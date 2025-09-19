# **Clanopedia**

**Community-governed knowledge database on ICP**
Built on top of **Blueband**, a Rust-based vector database on ICP, Clanopedia adds a **governance layer** for document embedding, curation, and lifecycle management.

Communities, DAOs, and organizations can use Clanopedia to **collect, search, and govern knowledge collaboratively**, with flexible models ranging from token voting to multisig committees.

---

## Features

* **Governance for Knowledge**

  * Token-based voting, multisig approvals, or SNS integration
  * Permissionless mode for trusted teams

* **Vector-powered Search**

  * Documents embedded into Blueband for semantic search
  * Support for text, markdown, YouTube transcripts, and GitHub commits

* **Flexible Document Pipelines**

  * Upload PDFs, DOCX, TXT, MD
  * Extract from URLs (YouTube playlists, GitHub repositories readmes)
  * Automated chunking + metadata preservation

* **Operations & Lifecycle**

  * Proposal system for embedding, updates, and deletion
  * Backup + migration tools to move collections between canisters or ICP accounts
  * Cycles tracked per collection with health monitoring

---

## Architecture

```
Clanopedia (Governance & Docs)  ←→  Blueband (Vector Storage & Search)
            ↑
       Frontend (UI/UX)
```

* **Clanopedia**: Handles governance, proposals, document flow, and cycle accounting
* **Blueband**: Stores documents + embeddings, powers semantic search
* **Frontend**: Community dashboard, query UI, and analytics

---

## 📌 Next Roadmap

**Milestone 1 – Governance + Dashboard**

* Org dashboards with query + cycle analytics
* Direct SNS integration for proposals
* Hosting kit with deployment + operator tools

**Milestone 2 – Audio Indexing + Public Demo**

* Audio embedding pipelines (YouTube + local media)
* Public indexing campaign (YouTube, GitHub, or archives)
* Developer APIs + docs for integration




## **Core Design Tables**

### **Table 1: Collection Configuration**
| Field                    | Type                | Description                                                    |
| ------------------------ | ------------------- | -------------------------------------------------------------- |
| `id`                     | String              | Unique identifier (format: col_{creator}_{timestamp}_{random}) |
| `name`                   | String              | Human-readable name                                            |
| `description`            | String              | Purpose description                                            |
| `admins`                 | Vec\<Principal\>    | Admin principals array                                         |
| `threshold`              | u32                 | Multisig threshold (e.g., 3 of 5)                              |
| `governance_token`       | Option\<Principal\> | ICRC-2 token canister for token-based voting                   |
| `governance_model`       | GovernanceModel     | One of: TokenBased, Multisig, SnsIntegrated, Permissionless    |
| `quorum_threshold`       | u32                 | Percentage (0-100) for token voting                            |
| `is_permissionless`      | bool                | Skip voting if true                                            |
| `blueband_collection_id` | String              | Linked Blueband collection ID                                  |
| `cycles_balance`         | u64                 | Deposited cycles for operations                                |
| `creator`                | Principal           | Collection creator (always an admin)                           |
| `created_at`             | u64                 | Creation timestamp                                             |
| `updated_at`             | u64                 | Last update timestamp                                          |
| `proposal_counter`       | u64                 | Counter for proposal IDs                                       |
| `active_proposals`       | HashMap             | Currently active proposals                                     |
| `proposals`              | Vec\<String\>       | Historical proposal IDs                                        |

### **Table 2: Governance Models**
| Model              | Description                              | Use Case                                    |
| ------------------ | ---------------------------------------- | ------------------------------------------- |
| **TokenBased**     | Token holders vote with their balance    | Democratic communities with token ownership |
| **Multisig**       | Admin votes with equal weight            | Committee-based governance                  |
| **SnsIntegrated**  | External SNS canister controls decisions | Enterprise/SNS-governed communities         |
| **Permissionless** | No voting required, direct admin actions | Trusted teams, simple collections           |

### **Table 3: Proposal Types**
| Proposal Type           | Parameters                  | Description                        |
| ----------------------- | --------------------------- | ---------------------------------- |
| `BatchEmbed`            | document_ids: Vec\<String\> | Embed multiple documents           |
| `ChangeGovernanceModel` | model: GovernanceModel      | Update collection governance model |
| `DeleteCollection`      | None                        | Delete entire collection           |
| `ChangeThreshold`       | new_threshold: u32          | Update multisig threshold          |
| `AddAdmin`              | admin: Principal            | Add new admin to collection        |
| `UpdateCollection`      | config: CollectionConfig    | Update collection settings         |
| `UpdateQuorum`          | new_percentage: u32         | Update token voting quorum         |
| `EmbedDocument`         | documents: Vec\<String\>    | Embed single document              |
| `RemoveAdmin`           | admin: Principal            | Remove admin from collection       |

### **Table 4: Document Operations**
| Operation                 | Description                                           | Governance Required?             |
| ------------------------- | ----------------------------------------------------- | -------------------------------- |
| `extract_from_file`       | Extract content from uploaded files (PDF, DOCX, etc.) | Yes (admin only)                 |
| `extract_from_url`        | Extract content from URLs (YouTube, GitHub)           | Yes (admin only)                 |
| `add_extracted_documents` | Add extracted documents to collection                 | Yes (varies by governance model) |
| `get_document`            | Retrieve document content                             | No (public)                      |

### **Table 5: Proposal Lifecycle**
| State      | Description                             | Duration       | Next States                 |
| ---------- | --------------------------------------- | -------------- | --------------------------- |
| `Active`   | Proposal is open for voting             | 7 days default | Approved, Rejected, Expired |
| `Approved` | Threshold met, ready for execution      | Until executed | Executed                    |
| `Rejected` | Threshold not met or explicit rejection | Permanent      | None                        |
| `Executed` | Successfully executed                   | Permanent      | None                        |
| `Expired`  | Time limit reached without threshold    | Permanent      | None                        |



## ⚖️ License

Clanopedia is open-source under the MIT License.

---

