"""Exercise process-output decoding under Windows' legacy locale."""
import json
import sys
import unittest
from unittest.mock import patch

from package_source import capture_utf8


class PackagingEncodingTests(unittest.TestCase):
    def test_cargo_metadata_preserves_unicode_with_a_windows_default_encoding(self):
        metadata = {"authors": ["Łukasz"], "description": "Shapes — colors"}
        payload = json.dumps(metadata, ensure_ascii=False).encode("utf-8")
        # This is real child-process output; the parent emulates Windows' ANSI locale.
        command = [sys.executable, "-c", "import sys; sys.stdout.buffer.write(bytes.fromhex('" + payload.hex() + "'))"]
        with patch("locale.getencoding", return_value="cp1252"):
            result = capture_utf8(command)
        self.assertEqual(json.loads(result), metadata)


if __name__ == "__main__":
    unittest.main()
