# Instruction bundle golden fixture

Synthetic data only. `bundle.txt` is raw UTF-8 (including a CRLF inside the rule,
a decomposed e+combining-acute, a CJK character and intentional blank lines).
It was framed from the ordered literal input list in the approved plan using
Python hashlib independently of the Rust renderer. `bundle.sha256` hashes every
byte in that file. Do not normalize line endings or regenerate it from actual
renderer output. The Rust test constructs the corresponding unsorted input.
