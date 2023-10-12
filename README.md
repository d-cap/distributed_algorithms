# Distributed algorithm
The repo contains example code after following courses on distributed algorithms:
- [CSE138 (Distributed Systems) lectures, Spring 2020](https://www.youtube.com/@lindseykuperwithasharpie)
- Distributed Algorithms for Message-Passing Systems 

## Reliable Broadcast Algorithm (Crash failure)
Allows to receive broadcast messages even with server crashes, the only condition needed is that the first message as already been sent, without that we cannot even start talking about broadcast messages.
> This algorithm is simulated using threads.

## Reliable Delivery Algorithm (Omission failure, not crash failure)
Allows to receive unicast messages when the network is not reliable.
> This algorithm is simulated using threads.

## Paxos consensus algorithm
Allows a group of n processes to decide on a value, the current implementation is not production ready in any way.

> This algorithm is simulated using different processes on the same machine, the roles are decided by the docker compose file
