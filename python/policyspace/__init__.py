"""PolicySpace's Python side: the client for the engine's decider protocol and the
project's deciders.

`protocol` reads and writes the protocol's messages (views, decisions, requests, replies,
recordings) in the engine's exact JSON form. `arithmetic` is the first decider, a rule that
turns a batch of views into a batch of decisions. `deciders` names the deciders and runs
the line-per-turn loop that carries one; `python -m policyspace <decider>` starts it.
"""
