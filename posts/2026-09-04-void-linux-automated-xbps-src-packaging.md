---
title: Automating the Void: Continuous Package Building with xbps-src and GitHub Actions
short_title: Automating xbps-src
subtitle: Continuous Void Linux Package Builds with GitHub Actions
date: 2026-09-04
slug: void-linux-automated-xbps-src-packaging
tags: void-linux, packaging, automation, ci-cd, sysadmin
---

# Automating the Void: Continuous Package Building with xbps-src and GitHub Actions

## Table of Contents
1. [Why Void Linux?](#why-void-linux)
2. [The Problem with Manual xbps-src Maintenance](#the-problem-with-manual-xbps-src-maintenance)
3. [Building the Upstream Watcher](#building-the-upstream-watcher)
4. [Containerized xbps-src in GitHub Actions](#containerized-xbps-src-in-github-actions)
5. [Automated Testing and Artifact Deployment](#automated-testing-and-artifact-deployment)
6. [Lessons Learned from Void Packaging](#lessons-learned-from-void-packaging)
7. [References](#references)

---

## Why Void Linux?

Void Linux is one of the most refreshing operating systems in existence. It has no systemd, boots in under 2 seconds with `runit`, uses the blindingly fast `xbps` package manager written in C, and supports both `glibc` and `musl` natively. We run Void across our personal workstations, edge routing nodes, and PowerEdge servers.

However, when you maintain private software or bleeding-edge packages on Void, you don't just download a `.deb` or `.rpm`. You write a `template` file in shell syntax and compile it from source using `xbps-src`:

```sh
# template excerpt for a custom tool
pkgname=custom-route-daemon
version=1.4.2
revision=1
build_style=cargo
short_desc="High performance BGP helper daemon"
maintainer="Zoa <zoa@zoa.sh>"
license="MIT"
homepage="https://github.com/vxfemboy/custom-route"
distfiles="https://github.com/vxfemboy/custom-route/archive/v${version}.tar.gz"
checksum="a84f39bc...982f"
```

Running `xbps-src pkg custom-route-daemon` builds the package inside a masterdir chroot and emits an architecture-specific `.xbps` binary ready for installation.

---

## The Problem with Manual xbps-src Maintenance

Building by hand works fine when you have two packages. But when you maintain an ecosystem of private daemons, custom kernel modules, and specialized utilities, manual packaging collapses:
1. Upstream developers release new versions. You have to notice, calculate new SHA256 checksums, edit the template, and test the build.
2. Cross-compilation across architectures (`x86_64`, `x86_64-musl`, `aarch64`) takes substantial local CPU time.
3. If an upstream dependency changes its ABI, silent breakage occurs at runtime.

We needed a fully automated continuous packaging pipeline that detects new upstream releases, bumps templates, compiles in clean chroots, and publishes them to our private repository.

---

## Building the Upstream Watcher

We created a GitHub Actions workflow that executes on a schedule to monitor upstream git repositories. When a new tag or GitHub release appears:

```yaml
name: Check Upstream Releases
on:
  schedule:
    - cron: '0 4 * * *'  # Daily at 4 AM UTC
  workflow_dispatch:

jobs:
  check-versions:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout custom-packages repo
        uses: actions/checkout@v4

      - name: Check latest releases and bump templates
        run: |
          python3 scripts/update_templates.py
```

The script extracts the latest release tag from GitHub APIs, downloads the tarball into a scratch space, computes the SHA256 checksum using `sha256sum`, and updates the template:

```python
# scripts/update_templates.py snippet
import re, hashlib, urllib.request

def bump_template(template_path, new_version, tarball_url):
    content = open(template_path).read()
    resp = urllib.request.urlopen(tarball_url)
    checksum = hashlib.sha256(resp.read()).hexdigest()
    
    content = re.sub(r'version=.*', f'version={new_version}', content)
    content = re.sub(r'revision=.*', 'revision=1', content)
    content = re.sub(r'checksum=.*', f'checksum={checksum}', content)
    
    with open(template_path, 'w') as f:
        f.write(content)
```

---

## Containerized xbps-src in GitHub Actions

Because `xbps-src` requires chroot capabilities, we run the compilation inside official Void Linux Docker containers:

```yaml
  build-package:
    needs: check-versions
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/void-linux/xbps-src-masterdir:latest
      options: --privileged
    steps:
      - name: Clone void-packages repo
        run: |
          git clone --depth=1 https://github.com/void-linux/void-packages.git /void-packages
          ./xbps-src binary-bootstrap

      - name: Build custom package
        run: |
          ./xbps-src -j$(nproc) -a x86_64 pkg custom-route-daemon
          ./xbps-src -j$(nproc) -a x86_64-musl pkg custom-route-daemon
```

---

## Automated Testing and Artifact Deployment

Once compiled, GitHub Actions signs the `.xbps` files using our private RSA repository key (`xbps-rindex --sign`) and uploads them via S3 to our public repository host.

On our Void Linux machines, updating is as simple as adding our repository to `/etc/xbps.d/`:
```text
repository=https://repo.zoa.sh/void/current
```
And running:
```bash
xbps-install -Su
```

No manual interventions. No broken chroots on local workstations.

---

## References
- [Void Linux Handbook](https://docs.voidlinux.org/)
- [xbps-src Source Packages Manual](https://github.com/void-linux/void-packages/blob/master/Manual.md)
- [XBPS Package Manager Reference](https://github.com/void-linux/xbps)
