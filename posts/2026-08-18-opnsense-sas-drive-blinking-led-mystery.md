---
title: The Anatomy of an OPNsense SAS Glitch: When Drive LEDs Lie
short_title: OPNsense SAS LED Mystery
subtitle: Tracking Down False Activity Lights on HardenedBSD
date: 2026-08-18
slug: opnsense-sas-drive-blinking-led-mystery
tags: bsd, opnsense, zfs, hardware, debugging, sysadmin
---

# The Anatomy of an OPNsense SAS Glitch: When Drive LEDs Lie

## Table of Contents
1. [The Setup: An OPNsense Router Running on Bare Metal](#the-setup-an-opnsense-router-running-on-bare-metal)
2. [The Symptom: The Blinking Light of Doom](#the-symptom-the-blinking-light-of-doom)
3. [SSHing into the FreeBSD Kernel](#sshing-into-the-freebsd-kernel)
4. [CAM, GEOM, and SAS Enclosure Services](#cam-geom-and-sas-enclosure-services)
5. [The Discovery: SES Passthrough and LED Signals](#the-discovery-ses-passthrough-and-led-signals)
6. [Formatting and Pool Creation](#formatting-and-pool-creation)
7. [References](#references)

---

## The Setup: An OPNsense Router Running on Bare Metal

Our primary edge gateway runs [OPNsense](https://opnsense.org/) (HardenedBSD/FreeBSD) directly on a bare-metal 1U chassis with an LSI SAS HBA (Host Bus Adapter) in IT mode (`mpr`/`mps` driver). 

The boot drive is an internal SATA SSD containing the root ZFS pool. To store extended packet capture ring buffers, NetFlow telemetry data, and local log archives without thrashing the boot SSD, we hot-plugged a second enterprise SAS 10K HDD into the front drive caddy.

The moment the drive latched into the backplane, the drive's green activity LED began flashing rapidly and erratically—and never stopped.

---

## The Symptom: The Blinking Light of Doom

In the storage world, a rapidly flashing drive LED usually signifies one of three things:
1. Intense I/O activity (unlikely for a blank drive).
2. Hardware drive failure / S.M.A.R.T. fault signal.
3. Drive rebuild or parity sync.

Yet, we hadn't mounted the drive, added it to any pool, or created a single filesystem. Was the drive defective, or was the SAS controller caught in an initialization loop?

---

## SSHing into the FreeBSD Kernel

We SSH'd directly into the router console (`root@10.0.0.1`) and checked the FreeBSD kernel message buffer via `dmesg`:

```text
mpr0: SAS Address from SAS device: 0x5000cca01a2b3c4d
da1 at mpr0 bus 0 scbus0 target 1 lun 0
da1: <HGST HUC109060CSS600 A400> Fixed Direct Access SPC-4 SCSI device
da1: 600000MB (1172123568 512 byte sectors)
```

The kernel recognized the drive cleanly as `da1` on the `mpr0` controller without a single SCSI sense error or timeout.

We queried the drive status using FreeBSD's Common Access Method (CAM) control utility:
```bash
camcontrol devlist -v
```

```text
scbus0 on mpr0 bus 0:
<HGST HUC109060CSS600 A400>         at scbus0 target 1 lun 0 (da1,pass1)
<Crucial CT500MX500SSD1 M3CR046>    at scbus0 target 0 lun 0 (da0,pass0)
<LSI SAS2008 09.00>                 at scbus0 target 8 lun 0 (ses0,pass2)
```

Both drives were present and responding normally to SCSI commands. So why were the front caddy LEDs blinking furiously?

---

## CAM, GEOM, and SAS Enclosure Services

To understand why the LED was blinking, you have to understand how enterprise drive bays signal status:
In standard desktop PCs, drive LEDs are wired directly to a motherboard header that pulses with disk activity.
In enterprise SAS servers, LEDs are controlled by **SES: SCSI Enclosure Services**.

The SAS backplane contains an expander chip (`ses0`) that listens to SCSI commands from the operating system. If an operating system sends an enclosure command to "locate" or "identify" a drive, the backplane firmware blinks the LED.

We checked the SES enclosure status using `sesutil`:
```bash
sesutil show
```

```text
ses0:
    Element 0: Power Supply
    Element 1: Fan
    Element 2: Drive Slot 0 (da0): Status OK
    Element 3: Drive Slot 1 (da1): Status Not Installed / Activity Blinking
```

The enclosure controller reported Slot 1 as having an active locate signal asserting. Why?

---

## The Discovery: SES Passthrough and LED Signals

The culprit turned out to be an artifact of FreeBSD's default SAS drive power management policy for spinning disks:
When a SAS disk is connected but has **zero active GEOM consumers** (no filesystem mounted, no active swap partition, no active ZFS pool importing it), the SAS HBA places the drive into a standby/unbound state. 

On this specific LSI SAS backplane firmware, an unbound SAS target triggers a continuous "Device Unconfigured" alert pulse on the front LED!

To prove this theory, we queried the drive's S.M.A.R.T. health using `smartctl`:
```bash
smartctl -H /dev/da1
```
```text
SMART Health Status: OK
Grown defect list: 0
Temperature: 32 C
```

The drive was completely healthy. The blinking was simply the backplane firmware complaining that the drive was sitting idle without being bound into a storage layer.

---

## Formatting and Pool Creation

We bound the drive into a dedicated secondary ZFS storage pool named `scratch`:

```bash
# Partition drive with GPT layout
gpart create -s gpt da1
gpart add -t freebsd-zfs -l pcap-storage da1

# Create ZFS pool with compression enabled
zpool create -o ashift=12 -O compression=lz4 -O atime=off scratch /dev/gpt/pcap-storage
```

The moment `zpool create` completed and GEOM registered the active consumer:
The frantic blinking stopped immediately. The LED transitioned to a solid, serene green, flashing only during actual disk reads and writes.

---

## Lessons Learned
- **Blinking enterprise LEDs don't always mean hardware failure**: Enterprise SAS backplanes communicate via SCSI Enclosure Services (SES), which can blink LEDs for unconfigured or unpartitioned disks.
- **Trust `camcontrol` and `sesutil` over visual indicators**: Always verify SCSI sense data in the kernel before assuming a drive is dying.

---

## References
- [FreeBSD CAM Subsystem Architecture](https://docs.freebsd.org/en/books/arch-handbook/driverbasics/)
- [OPNsense Documentation](https://docs.opnsense.org/)
- [SCSI Enclosure Services (SES-3) Specification](https://www.t10.org/drafts.htm)
