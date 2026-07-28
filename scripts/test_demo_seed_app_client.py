#!/usr/bin/env python
"""Tests the Flutter App bootstrap contract in the reusable demo seed."""

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))

from lib import demo_seed


class AppClientSeedTest(unittest.TestCase):
    def test_seeds_dedicated_client_and_registration(self) -> None:
        statements: list[str] = []

        with patch.object(demo_seed, "_sql_exec", statements.append):
            demo_seed._ensure_app_client_and_registration(None)

        self.assertEqual(len(statements), 1)
        sql = statements[0]
        self.assertIn("'fornetcode-app'", sql)
        self.assertIn("'[]'::jsonb, true", sql)
        self.assertIn("'registration', 'enabled', 'true'", sql)
        self.assertIn(
            "ON CONFLICT (realm_id, client_id) DO UPDATE",
            sql,
        )


if __name__ == "__main__":
    unittest.main()
