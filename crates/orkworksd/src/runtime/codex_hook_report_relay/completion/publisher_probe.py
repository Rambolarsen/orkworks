"""Test-only failure metadata; never print exception messages, paths or inputs."""
import runpy
import sys

script = sys.argv[1]
try:
    namespace = runpy.run_path(script)
    namespace["main"](sys.argv[1:])
except BaseException as error:
    stage, line = "load", 0
    trace = error.__traceback__
    while trace is not None:
        if trace.tb_frame.f_code.co_filename == script:
            stage = trace.tb_frame.f_code.co_name
            line = trace.tb_lineno
        trace = trace.tb_next
    category = "Invalid" if type(error).__name__ == "Invalid" else "OSError" if isinstance(error, OSError) else "other"
    number = error.errno if isinstance(error, OSError) and type(error.errno) is int else "none"
    sys.stderr.write(f"completion report failed; stage={stage}; exception={category}; errno={number}; line={line}\n")
    sys.exit(2)
