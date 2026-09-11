---
title: The Autonomous System Architecture, Part 1: The Fallacy of Stretched Clusters over WAN
short_title: The WAN Clustering Fallacy
subtitle: The Autonomous System Architecture, Part 1
date: 2026-08-14
slug: autonomous-system-part-1-wan-clustering-fallacy
tags: architecture, devops, distributed-systems, networking, sysadmin
series: Autonomous System Architecture
series_order: 1
---

# The Autonomous System Architecture, Part 1: The Fallacy of Stretched Clusters over WAN

*Part 1 of the Autonomous System Series: **Part 1: The Fallacy of Stretched Clusters over WAN** | [Part 2: Two-Tier IaC with Ansible and Standalone Incus →](/blog/autonomous-system-part-2-two-tier-iac-incus)*

## Table of Contents
1. [The Seductive Dream of Single-Pane-of-Glass Infrastructure](#the-seductive-dream-of-single-pane-of-glass-infrastructure)
2. [Distributed Consensus Meets Transatlantic Latency](#distributed-consensus-meets-transatlantic-latency)
3. [The Anatomy of a Cascading WAN Partition](#the-anatomy-of-a-cascading-wan-partition)
4. [The Coupling Tax: Why Less Is More](#the-coupling-tax-why-less-is-more)
5. [Designing for Island Survivability](#designing-for-island-survivability)
6. [Conclusion](#conclusion)
7. [References](#references)

---

## The Seductive Dream of Single-Pane-of-Glass Infrastructure

When you operate infrastructure across multiple disparate locations—say, a rack of Dell PowerEdge R710s in Kansas City (`kc`), an edge router in New York (`ny`), a compute node in Las Vegas (`lv`), and storage in Zurich (`ch`)—the siren song of modern DevOps starts playing in your ear:

> *"Why don't you just stretch a single cluster across all your sites? You can run an Incus cluster, or a single Kubernetes control plane! One command will manage everything. Single pane of glass!"*

It sounds elegant on a whiteboard. You run one CLI:
```bash
incus launch images:debian/12 my-app --target ny
```
And the cluster orchestrator takes care of scheduling, networking, and state replication.

We tried it. And we watched in horror as a transient transatlantic routing flap in Frankfurt brought down local container management in Missouri.

In this two-part post-mortem, I want to unpack exactly why **stretching distributed clustering protocols across public WAN links is fundamentally flawed**, and how we rebuilt our entire infrastructure around decoupled, autonomous nodes that cannot take each other down.

---

## Distributed Consensus Meets Transatlantic Latency

Modern clustered systems (Incus, Kubernetes, Nomad, Ceph) rely on consensus algorithms like **Raft** or **Paxos**, usually backed by distributed databases like `etcd` or `dqlite`.

Consensus protocols require a simple, unyielding mathematical condition to operate: **Quorum**.
For a cluster of $N$ nodes to commit any state change (creating a container, updating a route, restarting a service), a strict majority of nodes must agree:
$$Q = \left\lfloor \frac{N}{2} \right\rfloor + 1$$

In a local datacenter over 10GbE fiber, consensus is near-instantaneous. Latency is sub-millisecond, packet loss is negligible, and round-trips take microseconds:

```text
Datacenter LAN:
[Node A] <--- 0.2ms ---> [Node B] <--- 0.2ms ---> [Node C]
(Leader heartbeats succeed smoothly every 50ms)
```

Now take that same cluster and stretch it across the public Internet:

```text
Transatlantic WAN:
[KC: Node A] <======== 35ms ========> [NY: Node B]
     \                                    /
      \                                  /
     90ms                              85ms
        \                              /
         v                            v
               [CH: Node C (Zurich)]
```

When you introduce public WAN links into a consensus cluster:
1. **The speed of light sets a hard floor on write latency.** A write cannot complete until a round-trip to Zurich and New York finishes. What took 0.5ms on a LAN now takes 180ms minimum.
2. **Jitter disrupts leader heartbeats.** Raft uses periodic heartbeats (e.g. every 100ms–250ms) to maintain leadership. On the public Internet, BGP route recalculations or congestion can easily spike ping times by 200ms for a few seconds.
3. **The leader is declared dead.** The surviving nodes trigger an election. But elections require consensus across the high-latency mesh. During election cycles, **the entire cluster freezes and rejects API operations**.

---

## The Anatomy of a Cascading WAN Partition

Here is a real sequence of events extracted from our historical server logs during a transatlantic fiber cut:

```text
1. Normal operation:
   Nodes: KC (Leader), NY, LV, DE (Frankfurt), CH (Zurich). Quorum = 3 of 5.

2. Transatlantic fiber degradation:
   - Packet loss between North America and Europe spikes to 12%.
   - Latency jumps from 85ms to 240ms.

3. Dqlite heartbeat timeout:
   - Frankfurt (DE) stops receiving timely heartbeats from KC.
   - DE assumes KC has crashed and calls for a new election.

4. Distributed lockup:
   - Election votes cross the degraded WAN. Neither side reaches 3 votes in time.
   - The cluster enters a split-brain loop.
   - All Incus daemons across ALL hosts become unresponsive to local API calls.

5. Catastrophic local failure:
   - A sysadmin on KC types: `incus list` -> Hangs indefinitely.
   - Container healthchecks fail because the local daemon cannot read its state.
```

Notice the absurdity: **The Kansas City server had zero hardware issues and 100% local uptime.** Yet, because a fiber cable across the Atlantic Ocean was degraded, local containers in Kansas City could not be managed, restarted, or created!

This is the ultimate trap of tight distributed coupling: **it turns localized network hiccups into global catastrophic outages.**

---

## The Coupling Tax: Why Less Is More

We realized we were paying an exorbitant **coupling tax** for convenience we didn't actually need:

| Metric | Stretched WAN Cluster | Independent Autonomous Nodes |
|:---|:---|:---|
| **Blast Radius** | Global (one link down can freeze all nodes) | Strictly Local (failure stays on that box) |
| **Network Prerequisite** | Zero packet loss, low latency | Tolerates high latency, packet loss, total isolation |
| **Upgrades** | Lockstep, high-risk cluster migrations | Node-by-node rolling upgrades without downtime |
| **Complexity** | Distributed Raft/dqlite debugging | Standard Linux systemd/runit and local storage |

Why were we trying to turn five physical servers separated by thousands of miles into one giant virtual computer?

Our services didn't need distributed transactions across oceans:
- Our edge DNS and web reverse proxies are **stateless**. They just need a local configuration file and an anycast BGP announcement.
- Our mail server in Kansas City runs on **one machine**. It doesn't care whether Zurich is up or down.
- Our build runners in New York only need to run build jobs.

The whiteboard dream was an operational nightmare.

---

## Designing for Island Survivability

We dismantled the stretched cluster and adopted an architectural philosophy we call **Island Survivability**:

> **Rule of Island Survivability:**
> Every Point of Presence must be capable of running indefinitely as an isolated island, with full administrative capability, even if every other POP on Earth ceases to exist.

Under this rule:
1. **No distributed storage or databases over WAN.** Every host runs a local ZFS storage pool (`tank`) and a standalone Incus or Docker daemon.
2. **Local state only.** Local operations never query a remote database to authorize an action.
3. **Decoupled orchestration.** Centralized automation (Terraform, Ansible) drives deployments *into* the nodes, but the nodes never communicate with each other to maintain operational consensus.

---

## Conclusion

Clustering across a low-latency datacenter LAN is powerful. Clustering across the public Internet is hubris. 

By eliminating WAN consensus dependencies, we restored rock-solid stability to our infrastructure. If New York experiences a DDoS attack, Kansas City doesn't even notice. If Zurich loses power, Las Vegas keeps serving traffic without blinking.

In **[Part 2: Two-Tier IaC with Ansible and Standalone Incus →](/blog/autonomous-system-part-2-two-tier-iac-incus)**, we break down the practical implementation: how we use Ansible for host golden configs and remote Terraform providers to manage independent Incus hosts with per-POP isolated MinIO state.

---

## References
- [The Raft Consensus Algorithm](https://raft.github.io/)
- [Incus Clustering Architecture](https://linuxcontainers.org/incus/docs/main/clustering/)
- [Jepsen: Analyses of Distributed Systems](https://jepsen.io/analyses)
- [Fallacies of Distributed Computing (L. Peter Deutsch)](https://en.wikipedia.org/wiki/Fallacies_of_distributed_computing)
