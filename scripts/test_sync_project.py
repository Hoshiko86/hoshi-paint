import contextlib
import io
import os
import unittest
from unittest.mock import patch

import sync_project


class ProjectSyncTests(unittest.TestCase):
    def run_sync(self, dry_run, existing, done_option=True):
        mutations = []
        project = {
            "id": "project", "title": "Test",
            "fields": {
                "nodes": [{"id": "status", "name": "Status", "options":
                    [{"id": "done", "name": "Done"}] if done_option else []}],
                "pageInfo": {"hasNextPage": False},
            },
        }

        def graphql(query, token, **variables):
            self.assertEqual(token, "project-test-credential")
            if "fields(first:" in query:
                return {"user": {"projectV2": project}}
            if "items(first:" in query:
                return {"node": {"items": {"nodes": existing,
                    "pageInfo": {"hasNextPage": False, "endCursor": None}}}}
            mutations.append(variables)
            if "addProjectV2ItemById" in query:
                return {"addProjectV2ItemById": {"item": {"id": "new-item"}}}
            if "updateProjectV2ItemFieldValue" in query:
                return {"updateProjectV2ItemFieldValue": {"projectV2Item": {"id": "new-item"}}}
            self.fail("Unexpected GraphQL operation")

        def api(arguments, token):
            self.assertEqual(token, "repository-test-credential")
            return [[
                {"number": 1, "node_id": "open", "state": "open"},
                {"number": 2, "node_id": "closed", "state": "closed"},
                {"number": 3, "node_id": "pr", "state": "open", "pull_request": {}},
            ]]

        environment = {
            "GITHUB_ACTIONS": "true", "HOSHI_PROJECTS_TOKEN": "project-test-credential",
            "REPOSITORY_TOKEN": "repository-test-credential", "PROJECT_OWNER": "owner",
            "PROJECT_NUMBER": "2", "GITHUB_REPOSITORY": "owner/repository",
            "DRY_RUN": "true" if dry_run else "false",
        }
        with patch.dict(os.environ, environment, clear=True), \
                patch.object(sync_project, "graphql", side_effect=graphql), \
                patch.object(sync_project, "api", side_effect=api), \
                contextlib.redirect_stdout(io.StringIO()):
            sync_project.main()
        return mutations

    def test_dry_run_never_mutates_and_ignores_null_content(self):
        self.assertEqual(self.run_sync(True, [{"content": None}]), [])

    def test_add_missing_issue_and_mark_closed_done_preserving_open_status(self):
        existing = [{"id": "open-item", "content": {"id": "open"},
                     "fieldValueByName": {"optionId": "in-progress"}}]
        mutations = self.run_sync(False, existing)
        self.assertEqual(len(mutations), 2)
        self.assertEqual(mutations[0]["content"], "closed")
        self.assertEqual(mutations[1]["item"], "new-item")
        self.assertEqual(mutations[1]["option"], "done")

    def test_repeat_sync_does_not_duplicate_or_reset_statuses(self):
        existing = [
            {"id": "open-item", "content": {"id": "open"},
             "fieldValueByName": {"optionId": "in-progress"}},
            {"id": "closed-item", "content": {"id": "closed"},
             "fieldValueByName": {"optionId": "done"}},
        ]
        self.assertEqual(self.run_sync(False, existing), [])

    def test_missing_done_option_stops_before_mutations(self):
        with self.assertRaisesRegex(RuntimeError, "Done"):
            self.run_sync(False, [], done_option=False)


if __name__ == "__main__":
    unittest.main()
