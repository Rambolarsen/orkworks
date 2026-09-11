# Follow your sessions

Start with one session, then add another when you have independent work to do.
The sessions list keeps their coding tools and attention states visible while
you work in one terminal at a time.

## Switch without stopping the work

Select a session to return to its terminal. Switching away does not stop its
process: other sessions keep running while the backend remains alive. Recent
terminal history is replayed when you return. Closing the application is
different; live processes do not survive a backend restart.

## Read the signals

Peon is OrkWorks’ AI observer. With a provider configured, it reads recent
terminal output and produces summaries and attention signals. Some coding
tools can also report events through an explicitly installed integration.

Use a signal such as waiting for input as a reason to inspect the session.
An inferred summary can be wrong; it is not proof that tests passed or work
is complete. Integration coverage varies by coding tool.

The session details keep its working directory and Git context near the
conversation. OrkWorks shows context; your existing tools still create
branches, manage worktrees, and merge changes.

## Read a plan beside the terminal

When a session has an associated readable Markdown plan or specification,
choose **Review plan** in its Details card. The document opens in the reusable
Review tab beside Terminal. **Request independent review** sends a fixed
review request into the live session only when you choose it.

## When a signal looks wrong

Check the terminal first, then the selected provider and coding-tool
integration in Settings. A quiet terminal, an unavailable provider, and an
agent waiting for input are different situations. Report reproducible
problems through [GitHub issues](https://github.com/Rambolarsen/orkworks/issues).
