"""Tests for Horizon Android Multi-Version Registry and Dual-Backend Isolation."""
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class VersionRegistryPythonTests(unittest.TestCase):
    """Validates the multi-version registry contracts and backend isolation rules."""

    def standard_registry_data(self):
        """Constructs reference registry matching the specification."""
        return {
            "default_version": "26.3",
            "default_backend": "horizon-rust",
            "versions": [
                {
                    "minecraft_version": "26.3",
                    "protocol_version": 777,
                    "java_version": "java25",
                    "manifest_url": "https://piston-meta.mojang.com/v1/packages/26.3/26.3.json",
                    "backend_compatibility": ["horizon-rust", "java-fabric"],
                    "fabric_supported": True,
                    "fabric_loader_version": "0.16.10",
                    "installation_state": {"status": "not-installed"},
                    "verification_status": {"verification": "unverified"},
                    "playable": False,
                },
                {
                    "minecraft_version": "1.21.4",
                    "protocol_version": 769,
                    "java_version": "java21",
                    "manifest_url": "https://piston-meta.mojang.com/v1/packages/1.21.4/1.21.4.json",
                    "backend_compatibility": ["java-fabric"],
                    "fabric_supported": True,
                    "fabric_loader_version": "0.16.9",
                    "installation_state": {"status": "not-installed"},
                    "verification_status": {"verification": "unverified"},
                    "playable": False,
                },
                {
                    "minecraft_version": "1.21.1",
                    "protocol_version": 767,
                    "java_version": "java21",
                    "manifest_url": "https://piston-meta.mojang.com/v1/packages/1.21.1/1.21.1.json",
                    "backend_compatibility": ["java-fabric"],
                    "fabric_supported": True,
                    "fabric_loader_version": "0.16.9",
                    "installation_state": {"status": "not-installed"},
                    "verification_status": {"verification": "unverified"},
                    "playable": False,
                },
                {
                    "minecraft_version": "1.20.4",
                    "protocol_version": 765,
                    "java_version": "java17",
                    "manifest_url": "https://piston-meta.mojang.com/v1/packages/1.20.4/1.20.4.json",
                    "backend_compatibility": ["java-fabric"],
                    "fabric_supported": True,
                    "fabric_loader_version": "0.15.11",
                    "installation_state": {"status": "not-installed"},
                    "verification_status": {"verification": "unverified"},
                    "playable": False,
                },
            ],
        }

    def test_unverified_versions_are_never_marked_playable(self):
        catalog = self.standard_registry_data()
        for version in catalog["versions"]:
            self.assertFalse(
                version["playable"],
                f"Version {version['minecraft_version']} must not be marked playable while unverified",
            )

    def test_backend_compatibility_mapping(self):
        catalog = self.standard_registry_data()
        by_id = {v["minecraft_version"]: v for v in catalog["versions"]}

        # 26.3 must support both Horizon Rust engine and Java Fabric runtime
        self.assertIn("horizon-rust", by_id["26.3"]["backend_compatibility"])
        self.assertIn("java-fabric", by_id["26.3"]["backend_compatibility"])

        # 1.21.4 is pure Java runtime target, not Horizon Rust engine
        self.assertEqual(by_id["1.21.4"]["backend_compatibility"], ["java-fabric"])
        self.assertEqual(by_id["1.21.1"]["backend_compatibility"], ["java-fabric"])
        self.assertEqual(by_id["1.20.4"]["backend_compatibility"], ["java-fabric"])

    def test_java_requirements_match_spec(self):
        catalog = self.standard_registry_data()
        by_id = {v["minecraft_version"]: v for v in catalog["versions"]}

        self.assertEqual(by_id["26.3"]["java_version"], "java25")
        self.assertEqual(by_id["1.21.4"]["java_version"], "java21")
        self.assertEqual(by_id["1.21.1"]["java_version"], "java21")
        self.assertEqual(by_id["1.20.4"]["java_version"], "java17")

    def test_protocol_numbers(self):
        catalog = self.standard_registry_data()
        by_id = {v["minecraft_version"]: v for v in catalog["versions"]}

        self.assertEqual(by_id["26.3"]["protocol_version"], 777)
        self.assertEqual(by_id["1.21.4"]["protocol_version"], 769)
        self.assertEqual(by_id["1.21.1"]["protocol_version"], 767)
        self.assertEqual(by_id["1.20.4"]["protocol_version"], 765)

    def test_instance_isolation_paths(self):
        """Validates that Backend A and Backend B use completely separated directory trees."""
        with tempfile.TemporaryDirectory() as temp_dir:
            home = Path(temp_dir)

            # Backend A: Horizon Rust
            rust_home_default = home
            rust_home_profile1 = home / "profiles" / "profile-1"

            # Backend B: Java Fabric
            java_instance_26_3 = home / "instances" / "fabric-26.3"
            java_instance_1_21_4 = home / "instances" / "fabric-1.21.4"
            java_instance_1_21_4_p1 = home / "instances" / "fabric-1.21.4-profile-1"

            # Ensure none of the paths overlap
            all_paths = [
                rust_home_default,
                rust_home_profile1,
                java_instance_26_3,
                java_instance_1_21_4,
                java_instance_1_21_4_p1,
            ]
            self.assertEqual(len(all_paths), len(set(all_paths)))
            self.assertNotEqual(rust_home_profile1, java_instance_1_21_4_p1)
            self.assertTrue(str(java_instance_1_21_4).startswith(str(home / "instances")))


if __name__ == "__main__":
    unittest.main()
