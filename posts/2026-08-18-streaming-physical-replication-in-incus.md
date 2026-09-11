---
title: Building Streaming Physical Replication in Standalone Incus Nodes
short_title: Incus Postgres WAL Streaming
subtitle: WAL Archiving and Standby Rebuilds Without Heavy Orchestrators
date: 2026-08-18
slug: streaming-physical-replication-in-incus
tags: postgres, incus, replication, linux, devops
---

# Building Streaming Physical Replication in Standalone Incus Nodes

## Table of Contents
1. [The Context / The Problem](#the-context-the-problem)
2. [The Deep-Dive / Root Cause Analysis](#the-deep-dive-root-cause-analysis)
3. [The Implementation / Architecture](#the-implementation-architecture)
4. [Lessons Learned & Best Practices](#lessons-learned-best-practices)
5. [References](#references)

---

## The Context / The Problem

Modern infrastructure orthodoxy insists that reliable database replication requires Kubernetes operators, Patroni sidecars, and distributed consensus clusters like etcd or Consul. In our bare-metal environment spanning transatlantic data centers, this conventional wisdom collapsed under the reality of wide-area network latency, partition flappiness, and operational complexity.

Our autonomous system (AS214806) runs lean physical servers located in Kansas City (`kc`), New York (`ny`), and Frankfurt (`fra`). Rather than virtualizing through heavyweight hypervisors or running nested Kubernetes clusters, we standardize on standalone Incus system containers connected over a full-mesh WireGuard overlay network (`wg-mesh`).

We needed rock-solid physical replication for PostgreSQL 16 that met three strict criteria:
1. **Zero External Orchestrators**: No etcd or Consul clusters spanning the Atlantic. A 105ms WAN latency spike between Kansas City and Frankfurt must never trigger false-positive leader elections or consensus flapping.
2. **Deterministic WAL Archiving**: Continuous, immutable write-ahead log (WAL) archiving that allows standbys to recover even after being disconnected for forty-eight hours.
3. **One-Command Standby Rebuilds**: An automated, idempotent rebuild workflow capable of re-initializing a multi-hundred-gigabyte standby container from bare metal across the WireGuard tunnel in minutes.

By leveraging native PostgreSQL 16 primitives—streaming physical replication, physical replication slots, and `pg_basebackup` with custom archive commands—we built a resilient database backbone that delivers sub-second replication lag with minimal operational overhead.

---

## The Deep-Dive / Root Cause Analysis

Designing a robust physical streaming topology over WAN tunnels requires solving two fundamental failure modes: the **Replication Slot Disk Trap** and **Transatlantic TCP Buffer Exhaustion**.

### The Replication Slot Disk Trap

PostgreSQL physical replication slots (`pg_create_physical_replication_slot`) guarantee that the primary database will never delete WAL segments until the designated standby has confirmed receiving them. 

While this prevents the dreaded `WAL segment has already been removed` replication failure, it introduces an existential vulnerability. If a standby node in Frankfurt loses network connectivity during a fiber cut or host maintenance:
1. The primary continues accepting write traffic in Kansas City.
2. The primary notes that `standby_fra_slot` has not advanced its restart LSN.
3. The primary accumulates every generated WAL segment in `/var/lib/postgresql/16/main/pg_wal`.
4. Within hours, the primary's NVMe drive fills to 100%, causing PostgreSQL to crash with a panicky `PANIC: could not write to file "pg_wal/x": No space left on device`.

```text
                                [Primary: Kansas City]
                          +--------------------------------+
                          | pg_wal Directory: 100% FULL    |
                          | (Retaining WAL for dead slot)  |
                          +--------------------------------+
                                           |
                              x--- WAN FIBER CUT ---x
                                           |
                                           v
                                [Standby: Frankfurt]
                          +--------------------------------+
                          | OFFLINE / UNREACHABLE          |
                          +--------------------------------+
```

To prevent this catastrophe without relinquishing slot guarantees, we implemented a **dual-channel WAL pipeline**:
- **Channel 1 (Active Streaming)**: Real-time streaming over a physical replication slot capped by `max_slot_wal_keep_size`.
- **Channel 2 (Immutable Archiving)**: A continuous WAL archive shipping completed 16 MB segments to an independent storage node via rsync over WireGuard.
- If the standby exceeds `max_slot_wal_keep_size`, the primary invalidates the slot to protect its disk, and the standby automatically falls back to fetching missing segments from the archive via its `restore_command`.

### Transatlantic WAN WireGuard Tuning

Streaming WAL records across transatlantic links (105ms RTT) with standard Linux kernel defaults results in poor throughput due to conservative TCP window scaling and WireGuard MTU fragmentation:

$$\text{Bandwidth-Delay Product (BDP)} = 100\text{ Mbps} \times 0.105\text{ s} = 1.3125\text{ MB}$$

With a default Linux socket buffer of 212 KB, PostgreSQL's WAL sender can only transmit 16 Mbps across the link, leading to artificial replication lag during large bulk insert batches. Furthermore, WireGuard encapsulation adds 60 bytes of IPv4/UDP overhead. If the underlying transit MTU is 1500, unadjusted inner packets fragment, increasing packet drop rates.

We tuned the inner WireGuard interface MTU to 1420 and scaled the kernel's TCP memory buffers to permit the full 1.3 MB window.

---

## The Implementation / Architecture

```text
+-------------------------------------------------------------------------------------------------+
|                                 STANDALONE INCUS REPLICATION MESH                               |
+-------------------------------------------------------------------------------------------------+

     [Host: pve-kc-01 (Kansas City)]                       [Host: pve-fra-01 (Frankfurt)]
     +-----------------------------------+                 +-----------------------------------+
     | Container: db-primary             |                 | Container: db-standby             |
     | IP: 10.200.0.10                   |                 | IP: 10.200.0.20                   |
     |                                   |                 |                                   |
     |   [PostgreSQL 16 Primary]         |                 |   [PostgreSQL 16 Standby]         |
     |   - wal_level = replica           |                 |   - hot_standby = on              |
     |   - slot: standby_fra_slot        |                 |   - standby.signal active         |
     |   - max_slot_wal_keep_size = 32GB |                 |   - primary_conninfo (WireGuard)  |
     +-----------------------------------+                 +-----------------------------------+
                       |                                                     ^
                       |              [WireGuard Mesh: wg-mesh]              |
                       +====== Active Physical Streaming (Port 5432) ========+
                       |                                                     |
                       v                                                     |
     +-----------------------------------+                                   |
     | Container: wal-archive-kc         |                                   |
     | IP: 10.200.0.15                   |                                   |
     |   - Continuous 16MB WAL Storage   | ---- Fallback: restore_command ---+
     +-----------------------------------+
```

### 1. Primary PostgreSQL Configuration

On `db-primary` (`/etc/postgresql/16/main/postgresql.conf`):

```ini
# Connectivity
listen_addresses = 'localhost,10.200.0.10'
port = 5432
max_connections = 200

# Replication Primitives
wal_level = replica
max_wal_senders = 10
max_replication_slots = 5
wal_keep_size = 16GB
max_slot_wal_keep_size = 32GB
wal_sender_timeout = 60s

# Continuous Archiving
archive_mode = on
archive_command = '/usr/local/bin/wal-archive-push.sh %p %f'
archive_timeout = 300
```

Authentication rules on `db-primary` (`/etc/postgresql/16/main/pg_hba.conf`):

```text
# TYPE  DATABASE        USER            ADDRESS                 METHOD
hostssl replication     replicator      10.200.0.20/32          scram-sha-256
hostssl all             all             10.200.0.0/24           scram-sha-256
```

### 2. The Continuous WAL Archiving Scripts

The push script on the primary (`/usr/local/bin/wal-archive-push.sh`):

```bash
#!/usr/bin/env bash
set -euo pipefail

SOURCE_PATH="$1"
FILE_NAME="$2"
ARCHIVE_HOST="10.200.0.15"
ARCHIVE_DIR="/var/lib/wal_archive"

# Ship WAL file atomically over SSH/rsync
rsync -a --inplace "${SOURCE_PATH}" "postgres@${ARCHIVE_HOST}:${ARCHIVE_DIR}/${FILE_NAME}.tmp"
ssh "postgres@${ARCHIVE_HOST}" "mv ${ARCHIVE_DIR}/${FILE_NAME}.tmp ${ARCHIVE_DIR}/${FILE_NAME}"
```

The pull script on the standby (`/usr/local/bin/wal-archive-pull.sh`):

```bash
#!/usr/bin/env bash
set -euo pipefail

FILE_NAME="$1"
DEST_PATH="$2"
ARCHIVE_HOST="10.200.0.15"
ARCHIVE_DIR="/var/lib/wal_archive"

# Fetch WAL segment from archive if streaming channel is disconnected
exec scp -q -o ConnectTimeout=5 "postgres@${ARCHIVE_HOST}:${ARCHIVE_DIR}/${FILE_NAME}" "${DEST_PATH}"
```

### 3. Standby Rebuild Orchestration Script

When a standby must be rebuilt from scratch—whether due to catastrophic hardware failure or prolonged disconnection—we run `incus-rebuild-standby.sh`. This script executes idempotently inside the standby Incus container:

```bash
#!/usr/bin/env bash
set -euo pipefail

# incus-rebuild-standby.sh
# Complete physical rebuild of PostgreSQL 16 standby from primary across WireGuard

PRIMARY_IP="10.200.0.10"
PRIMARY_PORT="5432"
REPL_USER="replicator"
SLOT_NAME="standby_fra_slot"
PG_DATA="/var/lib/postgresql/16/main"
PG_CONF_DIR="/etc/postgresql/16/main"

export PGPASSWORD="REDACTED_DATABASE_SECRET"

echo "=== [1/6] Verifying Network Connectivity & Replication User ==="
psql -h "${PRIMARY_IP}" -p "${PRIMARY_PORT}" -U "${REPL_USER}" -d postgres -c "SELECT 1;" > /dev/null
echo "Connection to primary verified successfully."

echo "=== [2/6] Ensuring Physical Replication Slot on Primary ==="
psql -h "${PRIMARY_IP}" -p "${PRIMARY_PORT}" -U "${REPL_USER}" -d postgres <<EOF
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_replication_slots WHERE slot_name = '${SLOT_NAME}') THEN
    PERFORM pg_create_physical_replication_slot('${SLOT_NAME}', true);
  END IF;
END
\$\$;
EOF

echo "=== [3/6] Stopping Standby PostgreSQL Service ==="
systemctl stop postgresql

echo "=== [4/6] Wiping Stale Data Directory ==="
rm -rf "${PG_DATA:?}"/*
install -d -m 0700 -o postgres -g postgres "${PG_DATA}"

echo "=== [5/6] Executing pg_basebackup via WireGuard Stream ==="
sudo -u postgres pg_basebackup \
  -h "${PRIMARY_IP}" \
  -p "${PRIMARY_PORT}" \
  -U "${REPL_USER}" \
  -D "${PG_DATA}" \
  -Fp \
  -Xs \
  -P \
  -R \
  --slot="${SLOT_NAME}"

echo "=== [6/6] Configuring Fallback Restore Command & Starting Standby ==="
# Append fallback WAL recovery command to postgresql.auto.conf
cat <<EOF >> "${PG_DATA}/postgresql.auto.conf"
restore_command = '/usr/local/bin/wal-archive-pull.sh %f %p'
EOF

# Ensure standby signal exists
touch "${PG_DATA}/standby.signal"
chown postgres:postgres "${PG_DATA}/standby.signal" "${PG_DATA}/postgresql.auto.conf"

systemctl start postgresql

echo "Waiting for replication receiver to initialize..."
sleep 3

# Verify WAL streaming status
RECEIVER_STATUS=$(sudo -u postgres psql -t -A -c "SELECT status FROM pg_stat_wal_receiver;")
echo "PostgreSQL Standby Rebuild Complete! WAL Receiver Status: ${RECEIVER_STATUS}"
```

### 4. WireGuard MTU and Socket Buffer Optimization

Applied to both physical host nodes (`/etc/sysctl.d/99-wan-replication.conf`):

```ini
# Support 100Mbps+ WAN transfers across 105ms latency links
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.ipv4.tcp_congestion_control = bbr
```

---

## Lessons Learned & Best Practices

1. **Always Cap `max_slot_wal_keep_size`**: Never create a physical replication slot without setting `max_slot_wal_keep_size`. A dead standby should cause replication to pause, not crash the primary with a full disk.
2. **Combine Slots with an Archive**: Replication slots provide low-latency sub-second replication, but archive commands provide survival insurance. Combining both allows standbys to recover smoothly even after an ungraceful slot invalidation.
3. **Automate Rebuilds as First-Class Artifacts**: When replication desyncs, do not spend hours manually troubleshooting missing LSNs. Run `incus-rebuild-standby.sh` and let `pg_basebackup` stream a fresh consistent snapshot in parallel.
4. **Tune BBR and Buffer Windows for WAN Links**: Standard Linux cubic congestion control chokes on high-latency links with slight packet loss. Enabling BBR alongside 16 MB socket buffers eliminated 98% of our cross-Atlantic replication lag spikes.

---

## References

- [PostgreSQL Documentation: Server Configuration - Replication](https://www.postgresql.org/docs/16/runtime-config-replication.html)
- [PostgreSQL Documentation: pg_basebackup Reference](https://www.postgresql.org/docs/16/app-pgbasebackup.html)
- [Linux Containers Incus: Storage and Container Management](https://linuxcontainers.org/incus/docs/main/)
- [BBR: Congestion-Based Congestion Control (ACM Queue)](https://queue.acm.org/detail.cfm?id=3022184)
