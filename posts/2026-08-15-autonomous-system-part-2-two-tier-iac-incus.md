---
title: The Autonomous System Architecture, Part 2: Two-Tier IaC with Ansible and Standalone Incus
short_title: Two-Tier IaC with Incus
subtitle: The Autonomous System Architecture, Part 2
date: 2026-08-15
slug: autonomous-system-part-2-two-tier-iac-incus
tags: iac, ansible, incus, terraform, containers, sysadmin, devops
series: Autonomous System Architecture
series_order: 2
---

# The Autonomous System Architecture, Part 2: Two-Tier IaC with Ansible and Standalone Incus

*Part 2 of the Autonomous System Series: [← Part 1: The Fallacy of Stretched Clusters over WAN](/blog/autonomous-system-part-1-wan-clustering-fallacy) | **Part 2: Two-Tier IaC with Ansible and Standalone Incus***

## Table of Contents
1. [The Challenge: Global Automation Without Global Coupling](#the-challenge-global-automation-without-global-coupling)
2. [The Two-Tier Architecture Blueprint](#the-two-tier-architecture-blueprint)
3. [Tier 1: Host Golden-Config with Ansible](#tier-1-host-golden-config-with-ansible)
4. [Tier 2: Declarative Container Lifecycles with Terraform](#tier-2-declarative-container-lifecycles-with-terraform)
5. [State Isolation: Per-POP MinIO Backends](#state-isolation-per-pop-minio-backends)
6. [Secrets Management: SOPS + Age](#secrets-management-sops-plus-age)
7. [The Result: Real-World Resilience](#the-result-real-world-resilience)
8. [References](#references)

---

## The Challenge: Global Automation Without Global Coupling

In [Part 1](/blog/autonomous-system-part-1-wan-clustering-fallacy), we established why stretched WAN clusters (Raft, dqlite, etcd) inevitably collapse under real-world Internet packet loss and routing jitter. We locked in our core design constraint: **Island Survivability**. Every host must remain fully operational and manageable even if network connectivity between datacenters is severed.

However, rejecting distributed clusters creates a new challenge:
**How do you maintain declarative, automated, Infrastructure as Code (IaC) without manually SSHing into 8 different servers like a 1990s sysadmin?**

We wanted:
1. **Reproducible host configuration:** Operating systems, kernel sysctl parameters, WireGuard meshes, Bird BGP daemons, and ZFS storage pools defined as code in git.
2. **Declarative container management:** Containers, IP allocations, VLANs, and disk quotas declared cleanly in HCL rather than opaque bash scripts.
3. **Isolated blast radiuses:** A failure when provisioning a container in Frankfurt should never block, lock, or corrupt state for Kansas City or New York.

Here is the exact two-tier architecture we built to solve it.

---

## The Two-Tier Architecture Blueprint

We split infrastructure management into two distinct layers of abstraction:

```text
+-------------------------------------------------------------------------+
|                              Developer PC                               |
+-------------------------------------------------------------------------+
       |                                                    |
       | 1. Base OS, Kernel, Mesh, Incus Daemon             | 2. Containers, Networks, IPs
       v                                                    v
+-----------------------------+                     +-----------------------------+
|    Ansible (Golden Host)    |                     |   Terraform (Incus Workloads)   |
+-----------------------------+                     +-----------------------------+
       |                                                    |
       | (Direct SSH via WireGuard)                         | (HTTPS to Standalone Incus)
       v                                                    v
+-------------------------------------------------------------------------+
|                           Physical Bare-Metal                           |
|  - Host OS: Void Linux / Ubuntu Server                                  |
|  - ZFS Storage Pool: tank/incus                                         |
|  - Standalone Incus Daemon (local SQLite db, NO WAN cluster)           |
|  - WireGuard P2P Mesh + Bird BGP Router                                 |
|                                                                         |
|       [Container: mail]      [Container: dns]      [Container: proxy]   |
+-------------------------------------------------------------------------+
```

1. **Tier 1 (Substrate / Host):** **Ansible** provisions the physical metal. It installs packages, sets up kernel parameters, configures ZFS storage pools, builds our WireGuard mesh, and configures the standalone Incus daemon with a local, zero-network SQLite database.
2. **Tier 2 (Workloads / Containers):** **Terraform** manages the containers *inside* Incus. Instead of talking to a monolithic cluster API, Terraform defines remote endpoints targeting each independent Incus host directly.

---

## Tier 1: Host Golden-Config with Ansible

Ansible handles everything up to the point where the host is ready to accept container commands.

Here is an excerpt from our `roles/incus_host/tasks/main.yml` playbook:

```yaml
---
# Ensure kernel modules for container virtualization are loaded
- name: Load required kernel modules
  community.general.modprobe:
    name: "{{ item }}"
    state: present
  loop:
    - veth
    - xt_conntrack
    - overlay
    - bridge

# Tune kernel parameters for high-throughput container routing
- name: Apply sysctl network optimizations
  ansible.posix.sysctl:
    name: "{{ item.key }}"
    value: "{{ item.value }}"
    sysctl_set: true
    state: present
    reload: true
  loop:
    - { key: 'net.ipv4.ip_forward', value: '1' }
    - { key: 'net.ipv6.conf.all.forwarding', value: '1' }
    - { key: 'fs.inotify.max_user_instances', value: '1024' }
    - { key: 'fs.inotify.max_user_watches', value: '524288' }

# Install and initialize the standalone Incus daemon on local ZFS
- name: Ensure incus package is installed
  package:
    name: incus
    state: present

- name: Initialize standalone Incus daemon on local ZFS pool
  command: >
    incus admin init --auto
    --storage-backend=zfs
    --storage-pool=tank/incus
    --network-address=10.222.1.1
    --network-port=8443
  args:
    creates: /var/lib/incus/storage-pools/tank/incus

- name: Generate Incus remote authentication trust token
  command: incus config trust add terraform-deployer --quiet
  register: incus_trust_token
  changed_when: false
```

Notice the crucial parameter: `--network-address=10.222.1.1:8443`.
The Incus daemon binds its management API to its internal WireGuard mesh IP, protected by mutual TLS and token authentication. It runs purely on a local SQLite database (`/var/lib/incus/database/local.db`). There is zero distributed consensus.

---

## Tier 2: Declarative Container Lifecycles with Terraform

Now that the physical host exposes an authenticated Incus API over the private WireGuard mesh, we let **Terraform** handle container orchestration using the official [`lxc/incus`](https://registry.terraform.io/providers/lxc/incus/latest/docs) provider.

Here is our Terraform configuration for provisioning a micro-POP service:

```hcl
terraform {
  required_version = ">= 1.5.0"
  required_providers {
    incus = {
      source  = "lxc/incus"
      version = "~> 0.2.0"
    }
  }
}

# Provider configuration pointing to Kansas City standalone host
provider "incus" {
  alias   = "kc"
  address = "https://10.222.1.1:8443"
  token   = var.incus_kc_token
}

# Provider configuration pointing to New York standalone host
provider "incus" {
  alias   = "ny"
  address = "https://10.222.2.1:8443"
  token   = var.incus_ny_token
}

# Managed container instance on Kansas City
resource "incus_instance" "kc_mail_worker" {
  provider  = incus.kc
  name      = "mail-worker"
  image     = "images:debian/12"
  ephemeral = false

  config = {
    "boot.autostart"     = "true"
    "security.nesting"   = "true"
    "limits.cpu"         = "4"
    "limits.memory"      = "8GiB"
    "user.network-mode"  = "host-pinned"
  }

  device {
    name = "root"
    type = "disk"
    properties = {
      pool = "tank/incus"
      path = "/"
    }
  }

  device {
    name = "eth0"
    type = "nic"
    properties = {
      nictype        = "bridged"
      parent         = "incusbr0"
      "ipv4.address" = "10.222.100.25"
      "ipv6.address" = "fd00:b00b:1::25"
    }
  }
}

# Managed container instance on New York
resource "incus_instance" "ny_edge_proxy" {
  provider  = incus.ny
  name      = "edge-proxy"
  image     = "images:alpine/3.20"
  ephemeral = false

  config = {
    "boot.autostart"   = "true"
    "limits.cpu"       = "2"
    "limits.memory"    = "1GiB"
  }
}
```

By aliasing independent providers, a single `terraform apply` can declare instances across multiple nodes globally—yet each API request is an isolated, independent HTTPS call to that node's local daemon!

---

## State Isolation: Per-POP MinIO Backends

Where do we store Terraform state?

Many organizations use a single centralized Terraform state file for their entire fleet. This is an anti-pattern:
- A single state lock held during a slow deployment blocks all other deployments.
- If the central backend is unreachable due to a network partition, you cannot apply emergency fixes to unaffected nodes.

Instead, we **isolate state per Point of Presence** using lightweight S3-compatible [MinIO](https://min.io/) buckets hosted on our core nodes:

```hcl
terraform {
  backend "s3" {
    bucket                      = "tf-state-infrastructure"
    key                         = "pops/kc/terraform.tfstate"
    endpoint                    = "https://s3.kc.castletcp.internal:9000"
    region                      = "main"
    skip_credentials_validation = true
    skip_metadata_api_check     = true
    skip_region_validation      = true
    force_path_style            = true
  }
}
```

If Kansas City needs to be redeployed, it uses `pops/kc/terraform.tfstate`. If New York needs a container update, it uses `pops/ny/terraform.tfstate`. State locks are partitioned, and failure domains are strictly isolated.

---

## Secrets Management: SOPS + Age

We refuse to maintain external secrets daemons like HashiCorp Vault across our WAN just to decrypt API tokens and WireGuard keys.

Instead, we use [SOPS](https://github.com/getsops/sops) with [age](https://github.com/FiloSottile/age) asymmetric keys:

```text
secrets/
├── kc-secrets.enc.yaml
├── ny-secrets.enc.yaml
└── global-mesh.enc.yaml
```

Every server has its own age private key stored in `/etc/sops/age.key`. The public keys are committed to the git repository:

```yaml
# .sops.yaml
creation_rules:
  - path_regex: secrets/kc-.*\.yaml$
    key_groups:
      - age:
          - age1kc93k2sl94jf82m... # KC host key
          - age1adminzoa847xkd...  # Admin master key
  - path_regex: secrets/ny-.*\.yaml$
    key_groups:
      - age:
          - age1ny38dkx928skd... # NY host key
          - age1adminzoa847xkd...  # Admin master key
```

Secrets are version-controlled alongside playbooks. When Ansible runs, it decrypts only the secrets for the target host in memory using that host's local private key.

---

## The Result: Real-World Resilience

Since migrating to this Two-Tier architecture:
1. **Zero global cluster lockups:** Transatlantic fiber outages no longer impact local container orchestration.
2. **Instant recovery:** If a host's hard drive fails, Ansible recreates the golden substrate on new hardware in under 6 minutes, and `terraform apply` spins up all containers and restores local state.
3. **True multi-cloud flexibility:** The exact same Terraform patterns manage bare-metal R710 servers running Void Linux, Hetzner cloud instances in Germany, and Vultr edge nodes in Asia.

Keep your substrates dumb and independent. Keep your automation declarative and modular. Your future self on on-call duty will thank you.

---

## References
- [Ansible Documentation](https://docs.ansible.com/)
- [Incus Terraform Provider (lxc/incus)](https://github.com/lxc/terraform-provider-incus)
- [SOPS: Secrets OPerationS](https://github.com/getsops/sops)
- [age encryption tool](https://github.com/FiloSottile/age)
- [ZFS on Linux Documentation](https://openzfs.github.io/openzfs-docs/)
