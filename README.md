# Neobrew (`nbrew`)

A DX-focused, Homebrew-interoperable package manager for macOS and Linux,
written in Rust and powered by reactive pipelines built for extreme performance.

Neobrew is designed for zero-risk adoption and seamless interoperability with
Homebrew. It shares the official Homebrew filesystem state so that you can use
both of them interchangeably. It automatically delegates unsupported subcommands
to your local `brew`, though you may force a fallback by simply supplying
`--brew`.

> [!IMPORTANT]
>
> **Neobrew is currently under active development.** While the high-performance
> core architecture is steadily taking shape, the project is still undergoing
> heavy feature expansion. We are prioritizing rapid iteration and stability
> improvements over broad edge-case support as we push toward the **v0.1
> release**. Expect frequent updates as we refine the engine and deliver new
> capabilities.

## :zap: Architecture & Support Tiers :warning:

To accelerate toward the **v0.1 release** without compromising on the promises
of Neobrew, we have established the following architectural pillars and
strategic constraints.

### 1. Performance Architecture

Neobrew is engineered to maximize hardware utilization — achieving the highest
possible CPU throughput while maintaining an extremely low memory footprint.

- **Parallelism & Concurrency:** Neobrew orchestrates multiple parallel
  pipelines that concurrently handle downloads, hashing, and unpacking.
- **Zero-Copy Efficiency:** By utilizing shared buffers and memory-mapped files,
  Neobrew eliminates unnecessary copy overhead.
- **On-the-Fly Processing:** Multiple workers execute hash and unpack operations
  simultaneously as downloaded buffer is streamed, ensuring your softwares are
  ready the moment the download finishes.
- **Atomic & Lean Disk I/O:** All disk operations are atomic by default to
  prevent corrupted states from interrupted installs. Our storage layer is
  optimized for zero disk overhead to eliminate redundant space.
- **Content-Addressable Storage (CAS):** Neobrew utilizes CAS (content-addressed
  storage) to index and manage downloaded bottles and casks, ensuring
  high-integrity archive management and efficient file retrieval.

### 2. Ecosystem & Platform Support

Homebrew was originally born out of the need to provide the "missing package
manager for macOS." **Neobrew remains loyal to these roots.** Our mission is to
refine and provide a superior experience for Mac users, while also extending it
to the broader Linux ecosystem.

- **Target Audience:** We prioritize human developers on macOS and Linux
  workstations, as well as cloud-native environments and high-performance
  automation.
- **Cloud-Native & CI/CD:** While Neobrew is built with a human-centric DX and
  ergonomics in mind, its performance architecture is **cloud-native by
  design**. By utilizing parallelized processing and aggressive caching, it
  significantly slashes build times in **containerized environments** and
  **ephemeral CI/CD pipelines**.
- **Linux Support (Experimental):** While our current priority is the macOS
  ecosystem, we offer **experimental Linux builds** (aarch64 and x86_64) for
  power users on the cutting edge. We openly welcome feedback from the Linux
  community, though please note that macOS-specific issues are currently
  prioritized, since Neobrew is still in its very early days.
- **Support Tiers Cadence:** Neobrew closely tracks
  [Homebrew support tiers and schedules](https://docs.brew.sh/Support-Tiers) on
  a best-effort basis, governed by Neobrew's own architectural requirements,
  current feature parity, and future roadmap priorities.
- **Neobrew Support Tiers:**
    - **Tier 1 (Active Development):**
        - macOS ARM (Apple Silicon)
        - macOS x86 (Intel x86_64)
    - **Tier 2 (Experimental):**
        - Linux ARM (ARM64/AArch64)
        - Linux x86 (Intel x86_64)
        - WSL 2 ARM (ARM64/AArch64)
        - WSL 2 x86 (Intel x86_64)
    - **Tier 3 (Unsupported):**
        - Linux (all other architectures)
        - WSL (all other versions and architectures)

### 3. Default Prefix & Binary Support

To maintain peak performance, Neobrew relies on Homebrew’s pre-compiled
**Bottles** (binary packages) and **Casks**. These are built to function
specifically within the official default prefixes:

- **macOS ARM:** `/opt/homebrew`
- **macOS Intel:** `/usr/local`
- **Linux:** `/home/linuxbrew/.linuxbrew`

Following the official
[Homebrew Documentation on Installation](https://docs.brew.sh/Installation#untar-anywhere-unsupported):

> "Building from source is slow, energy-inefficient, buggy and unsupported. The
> main reason Homebrew just works is **because** we use bottles (binary
> packages) and most of these require using the default prefix. If you decide to
> use another prefix: don’t open any issues, even if you think they are
> unrelated to your prefix choice. They will be closed without response."

**Neobrew strictly enforces this rule.**

- **No Custom Prefixes:** We do not support installations in non-standard
  directory prefixes.
- **No Custom Source Builds:** Given our current fast iteration phase,
  **building from source (even on macOS) is not supported.** Neobrew is
  optimized for binary bottle and cask management to ensure performance and
  reliability.
- **Custom Taps & Binary Integrity:** Neobrew supports formulae and casks in
  Homebrew taps that provide precompiled bottles and casks, provided the SHA256
  of the bottle or cask matches the integrity checksum.

> [!NOTE]
>
> We are dedicated to building a faster, more ergonomic future for Mac/Linux
> package management. By staying within the standard installation paths, you
> allow us to spend less time on environment debugging and more time shipping
> the features you love.

### 4. Interoperability with Homebrew

Until Neobrew reaches greater maturity and stability, **it must run alongside an
official installation of Homebrew.**

- **Interoperability:** We strive for seamless reverse compatibility. You can
  use `nbrew` and `brew` interchangeably without fear of breaking your
  environment.
- **Command Forwarding:** Any subcommands not yet natively supported by Neobrew
  will be piped directly to binary of your underlying Homebrew installation.
- **Standard Environment:** Because Neobrew relies on the local Homebrew
  environment, only the official method of installing Homebrew is supported.

### 5. Filesystem-as-State (Unix Philosophy)

Like Homebrew, Neobrew treats the **filesystem as the source of truth.** Your
installed softwares are reflected entirely by the files and directories on your
disk, embracing the Unix philosophy: **"Everything is a file."**

- **Native Portability:** All state is self-describing and encoded within the
  filesystem. You can inspect, manage, back up, or restore your environment
  using standard Unix tools.
- **Transparent Debugging:** There is no discrepancy between what Neobrew
  "thinks" is installed and what is actually on disk — no database to query and
  no cache to invalidate.
- **Homebrew Harmony:** By sharing a common filesystem layout, `nbrew` and
  `brew` maintain a consistent, synchronized view of your environment, allowing
  for seamless coexistence without coordination overhead.

### :rocket: Community & Contributing :handshake:

We openly and warmly welcome ideas, issues, and discussions focused on improving
the DX and performance of package management on macOS and Linux.

- **Bug Reports:** If you find a bug in our Rust implementation, please open an
  issue! Bug fixes via PR are always welcomed and highly appreciated.
- **New Features:** If you have an idea for a feature or ergonomic improvement,
  **please open an issue or discussion thread first.** We would love to discuss
  the idea and implementation with you upfront to ensure it aligns with our
  current iteration goals before you commit to writing code.
