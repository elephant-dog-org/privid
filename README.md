# PrivID 🔐
**Decentralized Identity Verification Without Compromising Privacy**

PrivID is a browser extension that provides privacy-first identity verification by reading Soul-Bound Tokens (SBTs) from the Holonym protocol on Optimism blockchain. It enables users to prove identity attributes on Bluesky (AT Protocol) without exposing personal documents or biometric data.

> **Status**: ✅ **Real Blockchain Verification Implemented** | 🧪 Beta Testing
>
> **What Works**: Reads real SBTs from Optimism, verifies KYC/phone/passport/biometrics, posts to Bluesky
>
> **What's Beta**: Extension not yet submitted to Chrome/Firefox stores, UI/UX refinement ongoing

---

## 🚀 Quick Start

### Browser Extension (Recommended)

```bash
cd apps/extension
bun install
bun run build
# Load the extension in Chrome/Firefox (see apps/extension/README.md)
```

**Real Features**:
- 🔗 Connect Ethereum wallet (via signature authentication)
- ✅ Read real SBTs from Holonym Hub contract on Optimism
- 📊 Verify KYC, phone, passport, cleanHands, biometrics
- 🦋 Post verification to Bluesky via AT Protocol
- 🎨 Inject verification badge on Bluesky profiles

### Telegram Bot (Beta - Mock Mode)

```bash
cd apps/telegram-bot
cp env.example .env
# Add your TELEGRAM_BOT_TOKEN to .env
cargo run
```
**🧪 Beta**: Mock verification only, not connected to blockchain

### Web Frontend (Demo UI)

```bash
cd apps/frontend
bun install
bun run dev
# Visit http://localhost:5173
```
**🎨 Demo**: Visual showcase only, not connected to backend

---

## 🧭 Why PrivID?

Traditional identity verification often requires users to upload sensitive data to centralized services. This creates serious privacy concerns:

- **Third-Party Surveillance Tech**: Platforms like LinkedIn now use vendors such as [Persona](https://withpersona.com) to verify identity, requiring users to upload selfies and government-issued IDs. This data may be stored indefinitely. ([See critique](https://medium.com/@chrisbii/do-not-verify-your-identity-on-linkedin-4e0743854767))

- **Data Honeypots**: Centralized storage of identity documents and biometrics is a major security risk. Breaches can lead to identity theft and reputational harm.

- **Data Brokers and Loss of Control**: Tech giants like Google and Facebook monetize personal data at scale. Once uploaded, your ID may be shared, sold, or misused—without your consent.

PrivID offers a fundamentally different approach:
- Users stay in control of their data
- No centralized storage of identity documents
- Verifiable identity claims via **zero-knowledge proofs (ZKPs)** stored as SBTs on blockchain
- Read-only: PrivID reads existing verifications, doesn't create or store new ones

---

## 🔐 How PrivID Works

PrivID reads Soul-Bound Tokens (SBTs) from the Holonym protocol to verify identity claims—without revealing sensitive details.

**Workflow:**

1. **User Gets Verified**: Users first obtain SBTs from Holonym by verifying their identity (KYC, phone, passport, etc.) - this happens outside PrivID
2. **Connect Wallet**: User connects their Ethereum wallet to PrivID extension
3. **Read SBTs**: PrivID queries the Holonym Hub contract on Optimism blockchain to check for valid, non-revoked SBTs
4. **Verify Attributes**: Extension validates SBT expiry, revocation status, and credential type
5. **Display Badge**: If verified, PrivID shows a badge on the user's Bluesky profile
6. **Optional Post**: User can publish their verification status to Bluesky feed via AT Protocol

> 🔏 *You prove only what you want—nothing more. PrivID reads existing proofs, never stores personal data.*

---

## 🗂️ Repository Structure

This monorepo contains three main applications:

```
privid/
├── apps/
│   ├── extension/       # Browser extension ✅ REAL BLOCKCHAIN VERIFICATION
│   │                    # TypeScript, Vite, Bun, ethers.js
│   │                    # Reads SBTs from Optimism, wallet auth, Bluesky integration
│   │                    # Supports: KYC, phone, passport, cleanHands, biometrics
│   │
│   ├── telegram-bot/    # Telegram bot 🧪 BETA (Mock verification)
│   │                    # Rust, teloxide, tokio
│   │                    # Not yet connected to blockchain
│   │
│   └── frontend/        # Web application 🎨 DEMO ONLY
│                        # React, TypeScript, Vite, Tailwind CSS
│                        # UI showcase - not connected to backend
│
├── docs/                # Architecture and integration guides
├── archive/             # Archived contracts and legacy code
├── ROADMAP.md           # Project roadmap and milestones
├── SECURITY.md          # Security policy
└── CODE_OF_CONDUCT.md  # Community guidelines
```

---

## ✨ Features

### Browser Extension (Real Verification ✅)
**Functional with Real Blockchain Integration**

- **Wallet Connection**: Connect Ethereum wallet via signature authentication
- **Real SBT Verification**: Reads Soul-Bound Tokens from Holonym Hub contract on Optimism
- **Multiple Verification Types**:
  - 🆔 KYC Verification
  - 📞 Phone Number Verification
  - 🛂 Passport Verification
  - 🧼 Clean Hands Verification (anti-Sybil)
  - 👤 Biometrics Verification
- **SBT Validation**: Checks expiry dates and revocation status
- **Bluesky Integration**: Real AT Protocol authentication and posting
- **Badge Injection**: Visual verified badge on Bluesky profiles
- **Mock Mode**: Toggle for testing without wallet connection
- ⚠️ **Not yet submitted to Chrome/Firefox extension stores**

### Telegram Bot (Beta Testing 🧪)
**Mock Mode Only - Not Blockchain Connected**

- Privacy-respecting Telegram bot interface
- Mock verification flow (not real blockchain queries)
- Session management with file persistence
- Commands: `/start`, `/verify`, `/status`, `/help`
- ⚠️ **Awaiting blockchain integration**

### Web Frontend (Demo/Landing Page 🎨)
**UI Showcase Only**

- Visual demonstration of PrivID concept
- Interactive logo and animations
- Dark/Light mode support
- ⚠️ **Not connected to verification backend**

---

## 🧰 Tech Stack

### Browser Extension
- **Language**: TypeScript
- **Build Tool**: Vite
- **Package Manager**: Bun
- **Blockchain**: ethers.js v5, TypeChain
- **Network**: Optimism (L2)
- **Contract**: Holonym Hub at `0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB`
- **API**: @atproto/api (Bluesky/AT Protocol)
- **Wallet Auth**: Ethereum signature verification

### Telegram Bot
- **Language**: Rust (2021 Edition)
- **Framework**: teloxide 0.17.0
- **Async Runtime**: tokio 1.0

### Web Frontend
- **Framework**: React + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS, shadcn-ui
- **Platform**: Lovable

---

## 📍 Project Status & Roadmap

**Current Status: Real Blockchain Verification Implemented (Jan 2025)**

### What's Working ✅

#### Browser Extension - REAL VERIFICATION
- ✅ Ethereum wallet connection and authentication
- ✅ Query Holonym Hub contract on Optimism blockchain
- ✅ Read and validate Soul-Bound Tokens (SBTs)
- ✅ Support for 5 verification types (KYC, phone, passport, cleanHands, biometrics)
- ✅ SBT expiry and revocation checking
- ✅ Real Bluesky authentication and posting
- ✅ Badge injection on Bluesky profiles
- ✅ Mock mode toggle for testing

#### Telegram Bot - MOCK MODE
- ✅ Full command set functional
- ✅ Session management
- ⚠️ Mock verification only (not blockchain-connected)

#### Frontend - DEMO UI
- ✅ Visual interface
- ⚠️ Not connected to real verification

### What's Still Beta ⚠️

- Extension not submitted to Chrome/Firefox stores
- No mobile wallet support yet
- Telegram bot not connected to blockchain
- Frontend not integrated with backend
- Limited error handling for RPC failures
- No batch SBT queries (checks each type sequentially)

### Completed v0.1 Milestones (Issues 1-22)

- ✅ Browser extension scaffold and packaging
- ✅ **Real Holonym integration via blockchain** (PR #29)
- ✅ Badge rendering on Bluesky profiles
- ✅ ATProto publishing logic
- ✅ Telegram bot MVP (mock mode)
- ✅ Mock/test mode toggle
- ✅ Developer documentation
- ✅ **Wallet connection and authentication**
- ✅ **Ethereum/Optimism blockchain integration**
- ✅ **TypeChain contract typings**

For detailed roadmap, see: [ROADMAP.md](./ROADMAP.md)

### Next Steps (Post-MVP)

- Submit to Chrome/Firefox extension stores
- Add mobile wallet support (WalletConnect)
- Optimize SBT queries with batch requests
- Connect Telegram bot to blockchain
- Integrate frontend with real verification
- Custom AT Protocol schema (`app.privid.verification`)
- DID binding for portable identity
- Support additional blockchain networks
- Caching layer for SBT data

---

## 🔗 Related Projects & Dependencies

### Holonym Foundation
- [Holonym Protocol](https://holonym.io) – Privacy-preserving identity verification
- [Holonym Hub Contract](https://optimistic.etherscan.io/address/0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB) – SBT storage on Optimism
- [Holonym Documentation](https://docs.holonym.id) – Developer guides

### AT Protocol / Bluesky
- [AT Protocol](https://github.com/bluesky-social/atproto) – Decentralized protocol stack
- [Bluesky Social App](https://github.com/bluesky-social/social-app) – Reference client
- [@atproto/api](https://www.npmjs.com/package/@atproto/api) – AT Protocol SDK

### Blockchain Infrastructure
- [Optimism](https://optimism.io) – Layer 2 Ethereum network
- [ethers.js](https://docs.ethers.org/v5/) – Ethereum library
- [TypeChain](https://github.com/dethcrypto/TypeChain) – TypeScript bindings for Ethereum smart contracts

---

## 🔐 Holonym Integration

PrivID reads Soul-Bound Tokens (SBTs) from the Holonym Hub contract deployed on Optimism blockchain.

**Contract Address**: `0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB`

**Network**: Optimism (Chain ID: 10)

**RPC Endpoint**: `https://optimism-rpc.publicnode.com`

### Supported Verification Types

Each verification type has a unique circuit ID:

| Verification Type | Circuit ID | Description |
|------------------|-----------|-------------|
| KYC | `0x729d660e...` | Know Your Customer verification |
| Phone | `0xbce052cf...` | Phone number verification |
| Passport | `0xf2ce248b...` | Passport verification |
| Clean Hands | `0x1c98fc4f...` | Anti-Sybil verification |
| Biometrics | `0x0b512122...` | Biometric verification |

### How SBT Verification Works

1. User connects Ethereum wallet to extension
2. Extension queries `Hub.getSBT(address, circuitId)` for each verification type
3. Extension validates:
   - SBT exists
   - Not revoked (`revoked == false`)
   - Not expired (`expiry > current timestamp`)
4. If valid SBT found, user is marked as verified
5. Verification details stored locally in browser

> **Privacy Note**: PrivID only reads public SBT data from blockchain. No personal documents, biometrics, or identity details are accessed or stored by PrivID.

---

## 🌐 AT Protocol Integration

PrivID uses `@atproto/api` to authenticate users with their Bluesky handle and optionally post proof metadata to their feed.

**Current Implementation**:
- ✅ Bluesky authentication (handle + app password)
- ✅ Session management with JWT tokens
- ✅ Post verification proofs to user feeds
- ✅ Badge injection on Bluesky profiles (client-side)

**Verification Post Format**:
```
✅ Identity Verified with PrivID

Verification Type: [KYC/Phone/Passport/etc.]
Proof ID: real-sbt-proof-[random]
Timestamp: [ISO 8601]

Verified using zero-knowledge proofs via Holonym Protocol on Optimism blockchain.

#PrivID #ZeroKnowledge #Holonym
```

**Future Possibilities**:
- **Custom Schema**: Define `app.privid.verification` to store public proofs
- **Portable Identity**: Link verified claims to user's DID
- **Federated Verification**: Recognition across all AT Protocol applications

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork the repository**
2. **Choose an app** to work on (extension, frontend, or telegram-bot)
3. **Read the app-specific README** in the `apps/` directory
4. **Check existing issues** or create a new one
5. **Make your changes** following the code style of each project
6. **Test thoroughly**:
   - Extension: Use mock mode, then test with real wallet
   - Telegram bot: Ensure commands work
   - Frontend: Check UI/UX
7. **Submit a pull request** with a clear description

See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community guidelines.

---

## 🔒 Security & Privacy

PrivID is built with privacy as the foundation:

- **Read-Only**: Only reads public SBT data from blockchain, never writes or stores personal data
- **No Document Storage**: Never accesses or stores identity documents, biometrics, or credentials
- **Local-First**: Verification results stored only in browser local storage
- **No Backend**: Extension communicates directly with blockchain RPC and Bluesky API
- **Open Source**: All code is publicly auditable
- **Wallet Security**: Uses standard Ethereum signature authentication

**What PrivID Accesses**:
- ✅ Public SBT data on Optimism blockchain (revocation status, expiry, circuit ID)
- ✅ Bluesky public profile data
- ❌ Does NOT access: identity documents, biometrics, personal information, private keys

For security concerns, see [SECURITY.md](./SECURITY.md).

---

## 📚 Documentation

- [Architecture Overview](./docs/architecture.md)
- [Integration Guide](./docs/integration-guide.md)
- [Verification Flow](./docs/verification.md)
- [Extension README](./apps/extension/README.md)
- [Frontend README](./apps/frontend/README.md)
- [Telegram Bot README](./apps/telegram-bot/README.md)

---

## 🌟 Acknowledgments

PrivID integrates with:
- **[Holonym Foundation](https://holonym.io)** – For privacy-preserving identity SBTs
- **[Optimism](https://optimism.io)** – For scalable L2 blockchain infrastructure
- **[Bluesky/AT Protocol](https://github.com/bluesky-social)** – For decentralized social infrastructure
- **[Lovable](https://lovable.dev)** – For rapid frontend prototyping

---

## 📜 License

MIT License. See [LICENSE](./LICENSE) for details.

Open to collaboration and fork-friendly. Contributions welcome!

---

## 🆘 Support

- **Issues**: [GitHub Issues](https://github.com/elephant-dog-org/privid/issues)
- **Documentation**: See `docs/` directory
- **Questions**: Create a discussion or issue on GitHub

---

## ⚠️ Important Notes

**For Users**:
- You must first obtain SBTs from Holonym before using PrivID
- PrivID reads existing verifications, it doesn't create new ones
- You need an Ethereum wallet to connect to the extension
- Extension is in beta - not yet on Chrome/Firefox stores

**For Developers**:
- Extension requires Bun for building
- TypeChain generates contract typings from ABIs
- Mock mode available for development without wallet
- RPC endpoint can be changed in `blockchain/utils.ts`

---

**Built with privacy, powered by zero-knowledge proofs on the blockchain.**

PrivID - *You prove only what you want—nothing more.* 🔐
