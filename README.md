# KhamoshChat CLI

> Headless E2EE messaging client for Linux — built in pure Rust.

**`khamoshchat-client-cli`** is the official headless CLI companion for the [KhamoshChat](https://github.com/debarkamondal/khamoshchat-api) ecosystem. Designed for power users, AI agents, and GUI/tools that want E2EE messaging without a terminal UI.

## Features

- **End-to-end encryption** — Signal Protocol via `libsignal-dezire` (X3DH + Double Ratchet)
- **Two-tier SQLCipher storage** — encrypted primary DB + per-contact chat DBs, keys in system keyring
- **Async MQTT transport** — `rumqttc`, same topic structure as the mobile app
- **Google OAuth** — local redirect server, no browser popups required
- **Machine-friendly output** — every command supports `--json` for scripting and integration

## Install

```bash
# Requires: Rust 1.75+, sqlcipher-devel, libssl-devel
git clone https://github.com/debarkamondal/khamoshchat-client-cli.git
cd khamoshchat-client-cli
cargo build --release
sudo install target/release/khamoshchat /usr/local/bin/
```

## Quick Start

```bash
# 1. Authenticate with Google
khamoshchat auth

# 2. Add a contact
khamoshchat contacts add +919876543210 Alice

# 3. Send a message (E2EE)
khamoshchat send +919876543210 "Hello, world"

# 4. Fetch conversation history
khamoshchat history +919876543210 --limit 50

# 5. Check account status
khamoshchat status

# 6. List all conversations
khamoshchat list

# 7. Verify a contact's fingerprint
khamoshchat verify +919876543210
```

## Command Reference

| Command | Description |
|---|---|
| `auth [--no-open]` | Google OAuth + device registration |
| `list` | List all conversations |
| `history <contact> [--limit N] [--before ID]` | Fetch message history |
| `send <contact> <message>` | Send an E2EE message |
| `contacts add <phone> <name>` | Add a contact |
| `contacts list` | List all contacts |
| `verify <contact>` | Show safety-number fingerprint |
| `status` | Account + connection status |
| `daemon` | Long-lived MQTT listener → JSON Lines to stdout |

All commands accept `--json` for machine-readable output.

## Architecture

```
src/
├── main.rs          Entry point, tokio runtime
├── cli.rs           Clap argument parsing
├── client.rs        Command dispatcher
├── auth.rs          Google OAuth + tiny_http redirect server
├── keyring.rs       System keyring (Linux Secret Service)
├── storage/
│   ├── mod.rs       Store, PrimaryDb, ChatDb factory
│   ├── primary.rs   __primary__.db — account, contacts, inbox, outbox
│   └── chat.rs      {contact}.db — messages, ratchet sessions
├── crypto/
│   └── mod.rs       Signal Protocol (X3DH, Double Ratchet) via libsignal-dezire
└── mqtt/
    ├── mod.rs       MQTT client module
    └── client.rs    rumqttc event loop, topic: /khamoshchat/{recipient}/{sender}/
```

### Storage Design

```
System Keyring
├── khamoshchat/primary_db_key   →  __primary__.db (account, contacts, queues)
└── khamoshchat/chat_db_key       →  {contact}.db (messages, ratchet state)

Primary DB (__primary__.db)
├── account      → user_id, display_name, auth token
├── contacts     → phone, identity key, trust level
├── inbox        → raw ciphertext payloads
└── outbox       → pending sends (ciphertext-first)

Per-chat DB ({contact}.db)
├── messages     → direction, ciphertext, timestamp, status
├── ratchet_state → Double Ratchet session state
└── session_record → Serialized session
```

### Key Differences from Mobile

| Aspect | Mobile (React Native) | CLI |
|---|---|---|
| UI | React Native screens | Headless (stdout JSON Lines) |
| Secure storage | expo-secure-store | keyring (Linux Secret Service) |
| Database | expo-sqlite (RN bindings) | rusqlite + SQLCipher |
| MQTT | expo-native-mqtt | rumqttc (pure Rust) |
| Crypto | RN native module bridge | libsignal-dezire (direct Rust) |

## Requirements

- Linux (uses Linux Secret Service via `keyring` crate)
- Rust 1.75+
- `sqlcipher-devel` (Fedora/DNF: `dnf install sqlcipher-devel`)
- OpenSSL development headers (`openssl-devel`)

## Ecosystem

| Component | Repo |
|---|---|
| Backend API | [khamoshchat-api](https://github.com/debarkamondal/khamoshchat-api) |
| Mobile (iOS/Android) | [khamoshchat-mobile](https://github.com/debarkamondal/khamoshchat-mobile) |
| **CLI (this repo)** | **khamoshchat-client-cli** |

## License

GPL-3.0
