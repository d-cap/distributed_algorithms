# Distributed algorithm
The repo contains example code after following courses on distributed algorithms:
- [CSE138 (Distributed Systems) lectures, Spring 2020](https://www.youtube.com/@lindseykuperwithasharpie)
- Distributed Algorithms for Message-Passing Systems 

> Most of the algorithms are simulated using threads.

## Reliable Broadcast Algorithm (Crash failure)
Allows to receive broadcast messages even with server crashes, the only condition needed is that the first message as already been sent, without that we cannot even start talking about broadcast messages.

## Reliable Delivery Algorithm (Omission failure, not crash failure)
Allows to receive unicast messages when the network is not reliable.
