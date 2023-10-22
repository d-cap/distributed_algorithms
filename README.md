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

Allows a group of n processes to decide on a value, the current implementation was developed only to study the algorithm.

### Limitations

- Paxos roles in this implementation are hierarchical:
  - Proposer: is an acceptor and a learner as well;
  - Acceptor: is a learner as well;
  - Learner: is just a learner.
- The nodes should start from 1, if 0 is used the first propose will be ignored directly;

> This algorithm is simulated using different processes on the same machine, the roles are decided by the docker compose file

## Merkle tree

Data structure used to fast compare the content of the tree [Wikipedia](https://en.wikipedia.org/wiki/Merkle_tree). The current implementation is based on two arrays one sorted of the data key value pair and one that contains the actual hashes.

### Limitations

Because the data structure will be used to test how two servers can understand what data is not aligned in their database the current implementation has the following limitations:

- No deletion function is provided;
- Inserts must be in order to correctly have the hashes calculated. To allow this more complex code has to be developed and considering the goal of the current implementation is to test the data the functionality is not needed.
