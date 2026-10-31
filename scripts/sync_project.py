"""Manually invoked Actions helper. Never copy credentials from cloud machines."""

import json
import os
import subprocess


def api(arguments, token):
    environment = os.environ.copy()
    environment["GH_TOKEN"] = token
    result = subprocess.run(
        ["gh", "api", *arguments], env=environment, text=True, capture_output=True
    )
    if result.returncode:
        # Do not dump process environments, request headers or credential values.
        raise RuntimeError("GitHub API request failed. Check token scopes and Project access.")
    data = json.loads(result.stdout)
    if isinstance(data, dict) and data.get("errors"):
        raise RuntimeError("GitHub GraphQL returned errors; Project sync stopped.")
    return data


def graphql(query, token, **variables):
    args = ["graphql", "-f", f"query={query}"]
    for key, value in variables.items():
        flag = "-F" if isinstance(value, int) else "-f"
        args.extend([flag, f"{key}={value}"])
    return api(args, token)["data"]


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true":
        raise RuntimeError("Run this through the manual GitHub Actions workflow.")
    token = os.environ.get("HOSHI_PROJECTS_TOKEN")
    if not token:
        raise RuntimeError("Add the repository Actions secret HOSHI_PROJECTS_TOKEN first.")
    repository_token = os.environ["REPOSITORY_TOKEN"]
    owner = os.environ["PROJECT_OWNER"]
    number = int(os.environ["PROJECT_NUMBER"])
    dry_run = os.environ.get("DRY_RUN", "true").lower() != "false"
    query = """
      query($owner: String!, $number: Int!) {
        user(login: $owner) { projectV2(number: $number) {
          id title
          fields(first: 100) {
            nodes { ... on ProjectV2SingleSelectField { id name options { id name } } }
            pageInfo { hasNextPage }
          }
        } }
      }
    """
    project = graphql(query, token, owner=owner, number=number)["user"]["projectV2"]
    if project is None:
        raise RuntimeError("Project not found or not accessible to the PAT.")
    if project["fields"]["pageInfo"]["hasNextPage"]:
        raise RuntimeError("More than 100 Project fields; stop before making changes.")
    status = next((f for f in project["fields"]["nodes"] if f.get("name") == "Status"), None)
    done = next((o for o in status["options"] if o["name"] == "Done"), None) if status else None
    if done is None:
        raise RuntimeError('Project must have a Status field with a "Done" option.')

    items = {}
    cursor = None
    while True:
        query = """
          query($id: ID!, $cursor: String) {
            node(id: $id) { ... on ProjectV2 { items(first: 100, after: $cursor) {
              nodes { id content { ... on Issue { id } }
                fieldValueByName(name: "Status") {
                  ... on ProjectV2ItemFieldSingleSelectValue { optionId }
                }
              }
              pageInfo { hasNextPage endCursor }
            } } }
          }
        """
        variables = {"id": project["id"]}
        if cursor:
            variables["cursor"] = cursor
        page = graphql(query, token, **variables)["node"]["items"]
        for item in page["nodes"]:
            if (item.get("content") or {}).get("id"):
                items[item["content"]["id"]] = item
        if not page["pageInfo"]["hasNextPage"]:
            break
        cursor = page["pageInfo"]["endCursor"]

    repository = os.environ["GITHUB_REPOSITORY"]
    pages = api(
        [f"repos/{repository}/issues?state=all&per_page=100", "--paginate", "--slurp"],
        repository_token,
    )
    issues = [issue for page in pages for issue in page if "pull_request" not in issue]
    print(f'Project: {project["title"]}; issues: {len(issues)}; dry run: {dry_run}')
    for issue in issues:
        item = items.get(issue["node_id"])
        if item is None:
            print(f'Add issue #{issue["number"]}')
            if not dry_run:
                result = graphql(
                    """mutation($project: ID!, $content: ID!) {
                      addProjectV2ItemById(input: {projectId: $project, contentId: $content}) {
                        item { id }
                      }
                    }""",
                    token, project=project["id"], content=issue["node_id"],
                )
                item = result["addProjectV2ItemById"]["item"]
        current = (item or {}).get("fieldValueByName") or {}
        if issue["state"] == "closed" and current.get("optionId") != done["id"]:
            print(f'Mark issue #{issue["number"]} Done')
            if not dry_run:
                graphql(
                    """mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
                      updateProjectV2ItemFieldValue(input: {
                        projectId: $project, itemId: $item, fieldId: $field,
                        value: {singleSelectOptionId: $option}
                      }) { projectV2Item { id } }
                    }""",
                    token, project=project["id"], item=item["id"],
                    field=status["id"], option=done["id"],
                )
    print("Finished. Existing open-issue statuses and unrelated Project items were preserved.")


if __name__ == "__main__":
    main()
