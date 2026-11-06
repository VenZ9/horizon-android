import json
import os
import tempfile
import unittest
from pathlib import Path

STANDARD_ACTIONS = [
    ("action:key.jump", "Jump"),
    ("action:key.sneak", "Sneak"),
    ("action:key.sprint", "Sprint"),
    ("action:key.inventory", "Inventory"),
    ("action:key.chat", "Chat"),
    ("action:key.drop", "Drop"),
    ("action:key.attack", "Attack"),
    ("action:key.use", "Use"),
]

class TouchControlsTest(unittest.TestCase):
    def test_standard_actions_structure(self):
        self.assertEqual(len(STANDARD_ACTIONS), 8)
        tokens = [t[0] for t in STANDARD_ACTIONS]
        self.assertIn("action:key.jump", tokens)
        self.assertIn("action:key.attack", tokens)
        self.assertIn("action:key.use", tokens)
        self.assertIn("action:key.drop", tokens)
        self.assertIn("action:key.inventory", tokens)

    def test_settings_json_backward_compatibility(self):
        # A legacy JSON without key_overrides must still deserialize validly
        legacy_json = {
            "mode": "crosshair",
            "button_visibility": [0] * 14,
            "circle_size": 32.0,
            "custom_buttons": [
                {
                    "key": "action:key.jump",
                    "layouts": [
                        {"center": [0.5, 0.5], "size": 0.105, "opacity": 0.65},
                        {"center": [0.5, 0.5], "size": 0.105, "opacity": 0.65},
                        {"center": [0.5, 0.5], "size": 0.105, "opacity": 0.65},
                    ]
                }
            ],
            "layouts": [[{"center": [0.1, 0.1], "size": 0.1, "opacity": 0.65}] * 14] * 3
        }
        raw = json.dumps(legacy_json)
        loaded = json.loads(raw)
        self.assertNotIn("key_overrides", loaded)
        self.assertEqual(len(loaded["custom_buttons"]), 1)
        self.assertEqual(loaded["custom_buttons"][0]["key"], "action:key.jump")

    def test_control_layouts_path_structure(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            base = Path(tmpdir)
            ctl_dir = base / "control_layouts"
            ctl_dir.mkdir(parents=True, exist_ok=True)
            custom_json = ctl_dir / "custom.json"
            
            data = {
                "mode": "crosshair",
                "custom_buttons": [
                    {
                        "key": "key.keyboard.space",
                        "layouts": [{"center": [0.5, 0.5], "size": 0.105, "opacity": 0.65}] * 3
                    }
                ],
                "key_overrides": [None] * 14
            }
            data["key_overrides"][2] = "action:key.jump"
            custom_json.write_text(json.dumps(data, indent=2))
            
            self.assertTrue(custom_json.is_file())
            read_back = json.loads(custom_json.read_text())
            self.assertEqual(read_back["key_overrides"][2], "action:key.jump")
            self.assertEqual(read_back["custom_buttons"][0]["key"], "key.keyboard.space")

if __name__ == "__main__":
    unittest.main()
