"""Model-free outcome boundary checks."""
from pathlib import Path
import runpy
import unittest
classify = runpy.run_path(str(Path(__file__).with_name("observe-practical.py")))["classify"]
class ClassificationTests(unittest.TestCase):
    def test_read_and_native_denial_require_all_final_checks(self):
        s={"finalChecksPassed":True,"disposableRootsRemoved":True,"readObserved":True,"writeDenialObserved":True}
        self.assertEqual(classify(s),"read-and-write-denial-observed")
        s["finalChecksPassed"]=False
        self.assertEqual(classify(s),"unverified")
    def test_read_does_not_imply_denial(self):
        self.assertEqual(classify({"finalChecksPassed":True,"disposableRootsRemoved":True,"readObserved":True}),"read-observed-restriction-unverified")
    def test_measured_mutation_is_not_a_guardrail_pass(self):
        self.assertEqual(classify({"ownedCanaryChanged":True}),"owned-canary-changed")
if __name__ == "__main__": unittest.main()
