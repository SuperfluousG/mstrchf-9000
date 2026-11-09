//! Compiled maker registry. Transported records are exact copies, never evidence authority.
use crate::personal_scope::{PersonalUseScope, PORRIDGE_PERSONAL_SCOPE, AGLIO_PERSONAL_SCOPE, THREE_BEAN_PERSONAL_SCOPE, THREE_BEAN_TWO_UNIT_SCOPE, THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE, PORRIDGE_LARGER_SCOPE, THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE, CHILLED_CHICKEN_EXTRA_SCOPE, THREE_BEAN_SMALLER_SCOPE};
use mstrchf_contracts::*;
use mstrchf_core::recipes::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
mod courgette;
mod portable;
mod gf_chicken;
mod chilli;
mod overnight;
use crate::personal_scope::OVERNIGHT_OATS_SCOPE;
use crate::personal_scope::{CHILLI_BATCH_SCOPE,CHILLI_WRAPS_SCOPE,CHILLI_POTATO_SCOPE};
use crate::personal_scope::{COURGETTE_HOT_SCOPE, COURGETTE_COLD_SCOPE, PORTABLE_HOT_SCOPE, PORTABLE_COLD_SCOPE, THREE_BEAN_GF_CHICKEN_SCOPE, CHILLED_GF_CHICKEN_SCOPE};

const PACKET: &[u8] = include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/packet.json");
const DOCUMENTS: &[u8] = include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/evidence-documents.json");
const REGISTRY: &[u8] = include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/ingredient-registry.json");
const CAPTURES: &[u8] = include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/source-captures.json");

struct CompiledPacket {
    scope: PersonalUseScope,
    packet: &'static [u8], documents: &'static [u8], registry: &'static [u8], captures: &'static [u8],
    documents_digest: &'static str, registry_digest: &'static str, captures_digest: &'static str,
    policy_digest: &'static str, policy_id: &'static str, document_count: usize,
    method_id: &'static str, resource_id: &'static str,
}
const COMPILED: [CompiledPacket; 19] = [
    CompiledPacket { scope: PORRIDGE_PERSONAL_SCOPE, packet: PACKET, documents: DOCUMENTS, registry: REGISTRY, captures: CAPTURES,
        documents_digest:"ace4fab1283017f48d3911a2a79ccad1cedb6499dfe9e4eb51f2151b1d8a9a4f",
        registry_digest:"054556ac105133e16b373dd139d24f47fa361787ed1709cd18b2ace043801f59",
        captures_digest:"bb5c5d7986e08b89158050be250efc75b71fc3d255eeb58ef5a07bbb8e88b33f",
        policy_digest:"4e5f1e56dcf38c61f09e83e9a69ff7f82ca6d3245026eaaa3af7f355c3193885",
        policy_id:"ev_8000000000000000000000000000000b",document_count:11,
        method_id:"ev_8000000000000000000000000000000b",resource_id:"ev_8000000000000000000000000000000b" },
    CompiledPacket { scope: AGLIO_PERSONAL_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1/source-captures.json"),
        documents_digest:"89ab8a810da01c97aed09bf969f6ecd84c20c344303c0f6a628e6717d1f42f2d",
        registry_digest:"478ab6a465d2b61a069d8437349e61c5d3e32065b5c602a5860899c9dfda0390",
        captures_digest:"4a1ed643c2f3d447f330d1a420b6e52bfc771bf0269dd5f40bb7b24a2ab7911a",
        policy_digest:"ac952670abc629fae2be231e54672731920f98c2cd155cb29d5c23a7cba277f0",
        policy_id:"ev_80000000000000000000000000000019",document_count:10,
        method_id:"ev_80000000000000000000000000000019",resource_id:"ev_80000000000000000000000000000019" },
    CompiledPacket { scope: THREE_BEAN_PERSONAL_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/source-captures.json"),
        documents_digest:"50c3f8e08ecb662334ae27fde285d61f0c10f05b8b39cc1b57097224225a5170",
        registry_digest:"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1",
        captures_digest:"d34a70a50fa79aaad68ec534357e526ba08bf54ecd51752904525cea7fcc59db",
        policy_digest:"9ede3cecd23d0db345d2d63cab0049eaa75966abdd9bf2e06d6bc1053342b9a7",
        policy_id:"ev_80000000000000000000000000000025",document_count:6,
        method_id:"ev_80000000000000000000000000000024",resource_id:"ev_80000000000000000000000000000023" },
    CompiledPacket {scope:THREE_BEAN_TWO_UNIT_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-reviewed-v1/source-captures.json"),
        documents_digest:"aa9e4a3dbd4fe4b74e9b7c527e76e11c6fda0f1f035f25d033aab127389b7976",
        registry_digest:"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f",
        captures_digest:"a554a5f709f481cd965d4d9fecdd51236e96a355f62dad029f5f2067c8eed6dc",
        policy_digest:"bc2b8e8220b19a566542d211c3435ce2ef5d8004483697752e94c0df5b7bec71",
        policy_id:"ev_8000000000000000000000000000002b",document_count:6,
        method_id:"ev_8000000000000000000000000000002a",resource_id:"ev_80000000000000000000000000000029" },
    CompiledPacket {scope:THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/source-captures.json"),
        documents_digest:"e43d97aabc364ccc846724b222f0939c47c72288c9526c3205f64e51a7dc5f52",
        registry_digest:"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f",
        captures_digest:"6b6d2008b20d4d102c0bde05105b67d13f5ef9f877fc703fa3f8d59464fffe9b",
        policy_digest:"f3f83b809a55753e578ddae878363576d8e711b2b534d9c6727f3dad435fb68b",
        policy_id:"ev_80000000000000000000000000000032",document_count:7,
        method_id:"ev_80000000000000000000000000000030",resource_id:"ev_8000000000000000000000000000002f" },
    CompiledPacket {scope:PORRIDGE_LARGER_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-larger-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-larger-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-larger-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-larger-reviewed-v1/source-captures.json"),
        documents_digest:"3469948eecdeefc761c8701b61c8d83c2b97ba00259c6e6dbb066f4ca05ab31f",
        registry_digest:"0782d6aa6877ec07e3d5217a61a49dc50e4857b7744e394cb5991a1b583a2579",
        captures_digest:"bb5c5d7986e08b89158050be250efc75b71fc3d255eeb58ef5a07bbb8e88b33f",
        policy_digest:"ca6e8a80d20c331ab76765ac0df9aa3d247884003b923fd79d21668458552133",
        policy_id:"ev_8000000000000000000000000000003d",document_count:11,
        method_id:"ev_80000000000000000000000000000037",resource_id:"ev_80000000000000000000000000000037" },
    CompiledPacket {scope:THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-chicken-extra-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-chicken-extra-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-chicken-extra-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-two-unit-chicken-extra-reviewed-v1/source-captures.json"),
        documents_digest:"7b8fb8b087303d35654fc37fe918eb446b179aff2ce47804bd63405fcc7a3062",
        registry_digest:"204605d56020595ef9bb702c07d89d4a7125f7b8173fef6c4a71536d0248306f",
        captures_digest:"6b6d2008b20d4d102c0bde05105b67d13f5ef9f877fc703fa3f8d59464fffe9b",
        policy_digest:"9079dadd949edcd61da6a8c824e2b75ed598940ef019c74035c16d212ae4b3cb",
        policy_id:"ev_8000000000000000000000000000004c",document_count:7,
        method_id:"ev_8000000000000000000000000000004a",resource_id:"ev_80000000000000000000000000000049" },
    CompiledPacket {scope:CHILLED_CHICKEN_EXTRA_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilled-chicken-extra-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilled-chicken-extra-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilled-chicken-extra-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilled-chicken-extra-reviewed-v1/source-captures.json"),
        documents_digest:"2c3ee0af926882445b0fbc6878e63c2e9cd777cfc753ca4eb60d12db2c364c55",
        registry_digest:"30d530a39d08b30e5475c2c4bf5b35705ccfba6d7263c4e28ca96df9b21bf621",
        captures_digest:"74ef3f81a1a2e8e7de521832b37f6688176ebcdcfb13259da7044a14f5aebc37",
        policy_digest:"2f0e628670d1588644b58f477c4266d67be9af54872eab3eb95832b17aba4bd0",
        policy_id:"ev_80000000000000000000000000000045",document_count:9,
        method_id:"ev_80000000000000000000000000000043",resource_id:"ev_80000000000000000000000000000042" },
    CompiledPacket {scope:THREE_BEAN_SMALLER_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-smaller-reviewed-v1/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-smaller-reviewed-v1/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-smaller-reviewed-v1/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-smaller-reviewed-v1/source-captures.json"),
        documents_digest:"2c2c5367abd89264361a7226de8f8664cea027cc64b6b63681bd3fecd60313cf",
        registry_digest:"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1",
        captures_digest:"d34a70a50fa79aaad68ec534357e526ba08bf54ecd51752904525cea7fcc59db",
        policy_digest:"4cc91f07dcbbfef9031d6b8e98399655918aa9a3641194422ca4c83ea98d9bd7",
        policy_id:"ev_80000000000000000000000000000053",document_count:6,
        method_id:"ev_80000000000000000000000000000052",resource_id:"ev_80000000000000000000000000000051" },
    CompiledPacket {scope:COURGETTE_HOT_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-producer-packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/source-captures.json"),
        documents_digest:"a4fbea5b7d38a0f37b296271c5a6c2f355d1fe22043b109c8e58c3d4a437d9b1",
        registry_digest:"e6c8451dc8ecaedb336ddb40dedbb05a1f1a670061992393ad617d83391c618f",
        captures_digest:"6ee8cdc3c8b6f260fc2706a8afdbd88f84f250f3618490ea0e872bab1051122f",
        policy_digest:"0cbcb865ba807343c6901ab19b34dbc2729d0f76cb72fafae37cdc4744122c17",
        policy_id:"ev_8000000000000000000000000000005c",document_count:15,
        method_id:"ev_80000000000000000000000000000058",resource_id:"ev_80000000000000000000000000000059" },
    CompiledPacket {scope:COURGETTE_COLD_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-cold-service-packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/accepted-ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-home-leftovers-v1/source-captures.json"),
        documents_digest:"a4fbea5b7d38a0f37b296271c5a6c2f355d1fe22043b109c8e58c3d4a437d9b1",
        registry_digest:"e6c8451dc8ecaedb336ddb40dedbb05a1f1a670061992393ad617d83391c618f",
        captures_digest:"6ee8cdc3c8b6f260fc2706a8afdbd88f84f250f3618490ea0e872bab1051122f",
        policy_digest:"58a33066334ef31ae5004c7e5893cc98d00a67dba5060429b75d3e6a2b041be8",
        policy_id:"ev_80000000000000000000000000000062",document_count:15,
        method_id:"ev_8000000000000000000000000000005f",resource_id:"ev_80000000000000000000000000000060" },
    CompiledPacket {scope:PORTABLE_HOT_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-producer-packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/source-captures.json"),
        documents_digest:"6cd5675bf43c2225821475c9eba9cadeef6cc027d2f7cd04ab7189baf9f5eb62",
        registry_digest:"0551f2b5e3559076352adc098f185809cfbdbf8175a48eb9086c9ebcecd481fc",
        captures_digest:"112e5ff96dcc642c5392e2f66f413f4f4bbe0341e87de53e33b08687afeb6498",
        policy_digest:"a199064b293e60fbb0ffafb309f57cb50f074241881cb82816cac46c381ff392",
        policy_id:"ev_8000000000000000000000000000006b",document_count:15,
        method_id:"ev_80000000000000000000000000000067",resource_id:"ev_80000000000000000000000000000068" },
    CompiledPacket {scope:PORTABLE_COLD_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-cold-service-packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/accepted-ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/courgette-portable-reviewed-v1/source-captures.json"),
        documents_digest:"6cd5675bf43c2225821475c9eba9cadeef6cc027d2f7cd04ab7189baf9f5eb62",
        registry_digest:"0551f2b5e3559076352adc098f185809cfbdbf8175a48eb9086c9ebcecd481fc",
        captures_digest:"112e5ff96dcc642c5392e2f66f413f4f4bbe0341e87de53e33b08687afeb6498",
        policy_digest:"c222984f899aa1b0e649d47a7bfa238a2fb2065bd54992001cd02c5694538178",
        policy_id:"ev_80000000000000000000000000000071",document_count:15,
        method_id:"ev_8000000000000000000000000000006e",resource_id:"ev_8000000000000000000000000000006f" },
    CompiledPacket {scope:THREE_BEAN_GF_CHICKEN_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/base/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/base/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/base/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/base/source-captures.json"),
        documents_digest:"e946bd4e1c2905d1cd95779138ceb8dc291ebb0f855a82f0dc4b240e5ff69948",
        registry_digest:"5990284dc02af3651f97d40d22a29643c644363dd0f6da1e8d6318f653d4c124",
        captures_digest:"99c4fbf699feef6f64973eea9d94ec901fb110883199c85b4c926a77519fc8c1",
        policy_digest:"68303faae483a30f6606d9a24e3d5b0b4d84d5afa4f206e797ca87b254dd2022",
        policy_id:"ev_800000000000000000000000000000b9",document_count:12,
        method_id:"ev_800000000000000000000000000000b5",resource_id:"ev_800000000000000000000000000000b4" },
    CompiledPacket {scope:CHILLED_GF_CHICKEN_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/chicken/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/chicken/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/chicken/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-gf-chicken-extra-reviewed-v1/chicken/source-captures.json"),
        documents_digest:"4899c1f1c6364c315ea02d901464cb581354b792c8d913d4602b7bf7edcbf050",
        registry_digest:"5990284dc02af3651f97d40d22a29643c644363dd0f6da1e8d6318f653d4c124",
        captures_digest:"99c4fbf699feef6f64973eea9d94ec901fb110883199c85b4c926a77519fc8c1",
        policy_digest:"1792e5ff82fa835976eb355eac53455cb5d8b1ac36b0ef4a05e0a9153d744330",
        policy_id:"ev_800000000000000000000000000000c4",document_count:13,
        method_id:"ev_800000000000000000000000000000bf",resource_id:"ev_800000000000000000000000000000be" },
    CompiledPacket {scope:CHILLI_BATCH_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/producer/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/producer/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/producer/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/producer/source-captures.json"),
        documents_digest:"5caa57517aa00c9e98691d85955783ace7ba2a41bb5cd8de7bf010ec5408f64c",registry_digest:"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52",
        captures_digest:"c9e2acd065f444355a462e545e60d7cd1ac4523447a4d098d72449b41ccfba4e",policy_digest:"18ec303373378b1e32f8d81cbf59a5489439fc796ccb9ec1916519b3b69189fd",
        policy_id:"ev_800000000000000000000000000000d0",document_count:9,
        method_id:"ev_800000000000000000000000000000cc",resource_id:"ev_800000000000000000000000000000cd" },
    CompiledPacket {scope:CHILLI_WRAPS_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/wraps/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/wraps/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/wraps/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/wraps/source-captures.json"),
        documents_digest:"85e9f6f771c0458556beaa2ec9f12e4ec87fabd518ce8994bacfbd57079342f2",registry_digest:"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52",
        captures_digest:"c9e2acd065f444355a462e545e60d7cd1ac4523447a4d098d72449b41ccfba4e",policy_digest:"e45d0d5e773101a5efd9af8e8ccf272fad0c9b16037d1c91896c32c02c304c15",
        policy_id:"ev_800000000000000000000000000000d8",document_count:8,
        method_id:"ev_800000000000000000000000000000d5",resource_id:"ev_800000000000000000000000000000d6" },
    CompiledPacket {scope:CHILLI_POTATO_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/potato/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/potato/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/potato/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/chilli-two-remixes-reviewed-v1/potato/source-captures.json"),
        documents_digest:"c9138bec262a755932186c0bf7b528fc660bf8aca719f439163db8df3c8aad1e",registry_digest:"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52",
        captures_digest:"c9e2acd065f444355a462e545e60d7cd1ac4523447a4d098d72449b41ccfba4e",policy_digest:"69269a60a7a0e6c626779a9bc03736f59190924d84a92120bb4d7e74bfa90565",
        policy_id:"ev_800000000000000000000000000000e0",document_count:8,
        method_id:"ev_800000000000000000000000000000dd",resource_id:"ev_800000000000000000000000000000de" },
    CompiledPacket {scope:OVERNIGHT_OATS_SCOPE,
        packet:include_bytes!("../../../sites/mstrchf-9000/content/intake/peach-overnight-oats-reviewed-v1/breakfast/packet.json"),
        documents:include_bytes!("../../../sites/mstrchf-9000/content/intake/peach-overnight-oats-reviewed-v1/breakfast/evidence-documents.json"),
        registry:include_bytes!("../../../sites/mstrchf-9000/content/intake/peach-overnight-oats-reviewed-v1/breakfast/ingredient-registry.json"),
        captures:include_bytes!("../../../sites/mstrchf-9000/content/intake/peach-overnight-oats-reviewed-v1/breakfast/source-captures.json"),
        documents_digest:"8e3762b416668fd46c0b0186db5e25ecce891283237ea6d37fd28fc4321c4bef",
        registry_digest:"a15ee50cb51d98fbd468f23ebe4fa4c94aeff43362b9d69855a6ab59b88ca75f",
        captures_digest:"50b6e4b7415cb7ad4bd96af5dca079c81b28f2f09f8b929b5093a1698745ccba",
        policy_digest:"f4cc431620962df71627be23446a932ac02947d02570ca6cd42674f77c1e56c1",
        policy_id:"ev_800000000000000000000000000000e8",document_count:8,
        method_id:"ev_800000000000000000000000000000e5",resource_id:"ev_800000000000000000000000000000e6" },
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewedPacketRecord {
    pub packet: Value,
    pub evidence_documents: Value,
    pub ingredient_registry: Value,
    pub source_captures: Value,
}

#[derive(Debug)]
pub struct TrustedPacket {
    record: ReviewedPacketRecord,
    reference: ReviewedRecipePacketRef,
    recipe: RecipeRevision,
    basis: EvidenceBasis,
    classes: ComponentClassView,
    not_before: Instant,
    scope: PersonalUseScope,
    policy_id: EvidenceId,
    method_id: EvidenceId,
    resource_id: EvidenceId,
    relevant_essentials: Vec<IngredientId>,
    // None preserves the old whole-recipe resource projection byte-for-byte.
    phase_equipment: Option<Vec<(u16,Vec<EquipmentCode>)>>,
}
impl TrustedPacket {
    pub fn record(&self) -> &ReviewedPacketRecord { &self.record }
    pub fn reference(&self) -> &ReviewedRecipePacketRef { &self.reference }
    pub fn recipe(&self) -> &RecipeRevision { &self.recipe }
    pub(crate) fn basis(&self) -> &EvidenceBasis { &self.basis }
    pub(crate) fn classes(&self) -> &ComponentClassView { &self.classes }
    pub fn scope_id(&self) -> &'static str { self.scope.scope_id }
    pub fn scope(&self) -> &PersonalUseScope { &self.scope }
    pub fn root_policy_evidence_id(&self) -> &EvidenceId { &self.policy_id }
    pub(crate) fn production_method_evidence_id(&self) -> &EvidenceId { &self.method_id }
    pub(crate) fn phase_resource_evidence_id(&self) -> &EvidenceId { &self.resource_id }
    pub(crate) fn phase_equipment(&self, prep_id:u16) -> Result<&[EquipmentCode],DomainError> {
        if !self.recipe.prep.iter().any(|prep|prep.prep_id==prep_id) {return Err(fail(ErrorCode::MissingReference,"/maker/phase_resources/prep_id"));}
        match &self.phase_equipment {
            None => Ok(&self.recipe.equipment),
            Some(entries) => entries.iter().find(|(id,_)|*id==prep_id).map(|(_,equipment)|equipment.as_slice())
                .ok_or_else(||fail(ErrorCode::MissingReference,"/maker/phase_resources/prep_id")),
        }
    }
    /// Only the exact compiled portable service can supply these phase roles.
    pub(crate) fn portable_cold_method(&self) -> Option<mstrchf_core::food::PortableColdMethod> {
        if self.scope.scope_id == PORTABLE_COLD_SCOPE.scope_id {
            Some(mstrchf_core::food::PortableColdMethod {packing_prep_id:1,service_prep_id:2})
        } else {None}
    }
    pub(crate) fn relevant_essential_ingredient_ids(&self) -> &[IngredientId] { &self.relevant_essentials }
    pub fn not_before(&self) -> &Instant { &self.not_before }
    /// The exact compiled recipe-input definition, never observed product facts
    /// or a ProductMeaning/reader/policy authority. None for older packets.
    pub fn intrinsic_input_conditions(&self) -> Option<&Value> {
        self.record.packet.get("intrinsic_input_conditions")
    }
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PinDocument { evidence_id: EvidenceId, revision: Revision, digest: Digest }
fn fail(code: ErrorCode, path: &str) -> DomainError { DomainError::new(code, path) }
fn document(bytes: &[u8]) -> Result<Value, DomainError> {
    parse_document(bytes)
}
fn binding(value: &Value, expected: &str, path: &str) -> Result<(), DomainError> {
    if canonical_digest(value)?.0 != expected { return Err(fail(ErrorCode::DigestMismatch, path)); }
    Ok(())
}
const VEGETARIAN_INPUT_RULE: &str = "no animal body-part ingredients, or products/processing aids made from them; no meat/fish/bone stocks, carcass fats, gelatine or isinglass";
const VEGETARIAN_INPUT_DIGEST: &str = "cd344e3c63034ca967b2e559d6a5cc8541c9ee60cfe080e3364f0b7cb245f1e1";
const VEGETARIAN_REVIEW_DIGEST: &str = "9c4ae31e3266e8e85900f6075895f210111e33c89ba07b46b1a9bed6adb9857b";
fn validate_vegetarian_inputs(record: &ReviewedPacketRecord, recipe: &RecipeRevision,
    reference: &RecipeRef) -> Result<(), DomainError> {
    let packet=&record.packet;
    let table=&packet["intrinsic_input_conditions"];
    let documents=record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/intrinsic/documents"))?;
    let review=&documents[5]; let policy=&documents[6];
    let reference=serde_json::to_value(reference).map_err(|_|fail(ErrorCode::InvalidValue,"/maker/intrinsic/recipe"))?;
    let review_id="ev_80000000000000000000000000000031";
    let policy_id="ev_80000000000000000000000000000032";
    let requirement=serde_json::json!({"kind":"diet","data":{"code":"vegetarian"}});
    if table["format"]!="mstrchf-recipe-intrinsic-input-conditions" || table["version"]!=1
        || table["status"]!="maker_adopted_definition_not_product_observation"
        || table["recipe_reference"]!=reference || review["recipe_reference"]!=reference || policy["recipe_reference"]!=reference
        || table["rule"]!=VEGETARIAN_INPUT_RULE || table["dietary_review_id"]!=review_id || table["maker_policy_id"]!=policy_id
        || table["applies_even_when_all_consumers_confirmed_none"]!=true {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/intrinsic/recipe"));
    }
    binding(table,VEGETARIAN_INPUT_DIGEST,"/maker/intrinsic/digest")?;
    binding(review,VEGETARIAN_REVIEW_DIGEST,"/maker/intrinsic/dietary_review")?;
    let rows=table["entries"].as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/intrinsic/entries"))?;
    if rows.len()!=9 || recipe.ingredients.len()!=9 {return Err(fail(ErrorCode::MissingReference,"/maker/intrinsic/coverage"));}
    let expected=[("07",160,"g","drained"),("08",160,"g","drained"),("09",160,"g","drained"),
        ("0a",80,"g","raw"),("0b",40,"g","raw"),("0c",120,"g","raw"),
        ("0d",20,"ml","prepared"),("0e",30,"ml","prepared"),("0f",10,"g","prepared")];
    for (index,((row,ingredient),(suffix,amount,unit,form))) in rows.iter().zip(recipe.ingredients.iter()).zip(expected).enumerate() {
        let quantity=serde_json::json!({"amount":{"numerator":amount.to_string(),"denominator":"1"},"unit":unit,"form":form});
        if ingredient.ingredient_id.0!=format!("ig_800000000000000000000000000000{suffix}")
            || serde_json::to_value(&ingredient.quantity).unwrap()!=quantity
            || row["ingredient_id"]!=serde_json::to_value(&ingredient.ingredient_id).unwrap()
            || row["recipe_input_name"]!=serde_json::to_value(&ingredient.name).unwrap()
            || !ingredient.name.0.contains("vegetarian-compatible") || row["form"]!=form || row["unit"]!=unit
            || row["predicates"]!=serde_json::json!([{"kind":"vegetarian_compatible_input_composition","requirement":requirement},
                {"kind":"no_animal_body_part_ingredients_or_processing_products"}])
            || row["cooked_ready_to_eat_canned_required"]!=(index<3)
            || row["plain_unseasoned_input_required"]!=(index>=3)
            || row["actual_source_or_manufacturer_processing_evidence_status"]!="unknown_not_qualified"
            || row["not_inferred_from_title_brand_or_recipe"]!=true
            || !row["input_condition"].as_str().is_some_and(|text|text.contains(VEGETARIAN_INPUT_RULE)) {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/intrinsic/input"));
        }
    }
    let steps=serde_json::to_value(&recipe.steps).unwrap();
    let ingredients=serde_json::to_value(recipe.ingredients.iter().map(|ingredient|&ingredient.ingredient_id).collect::<Vec<_>>()).unwrap();
    if table["full_method_step_ids"]!=serde_json::json!([1,2]) || table["full_method_prep_ids"]!=serde_json::json!([1,2])
        || review["coverage"]["ingredient_ids"]!=ingredients
        || review["coverage"]["component_ids"]!=serde_json::json!(["co_80000000000000000000000000000006"])
        || review["coverage"]["step_ids"]!=table["full_method_step_ids"] || review["coverage"]["prep_ids"]!=table["full_method_prep_ids"]
        || review["coverage"]["all_ingredient_input_conditions"]!=table["entries"]
        || review["full_method"]!=steps || documents[4]["full_method"]!=steps
        || review["requirement"]!=requirement || review["verdict"]!="supported"
        || recipe.dietary_evidence.len()!=1
        || recipe.dietary_evidence[0].requirement!=(HardRequirement::Diet{code:DietCode::Vegetarian})
        || recipe.dietary_evidence[0].verdict!=DietaryVerdict::Supported
        || recipe.dietary_evidence[0].evidence_id.0!=review_id || recipe.dietary_evidence[0].review_id.0!=review_id {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/intrinsic/full_method"));
    }
    let digest=serde_json::json!(VEGETARIAN_INPUT_DIGEST);
    let review_pin=serde_json::json!({"evidence_id":review_id,"revision":"1","digest":VEGETARIAN_REVIEW_DIGEST});
    let policy_pin=serde_json::json!({"evidence_id":policy_id,"revision":"1","digest":"f3f83b809a55753e578ddae878363576d8e711b2b534d9c6727f3dad435fb68b"});
    if packet["intrinsic_input_conditions_digest"]!=digest || review["intrinsic_input_conditions_digest"]!=digest
        || policy["accepted_intrinsic_input_conditions_digest"]!=digest || policy["accepted_dietary_review_pin"]!=review_pin
        || packet["intrinsic_input_custody"]!=serde_json::json!({"recipe_reference":reference,"intrinsic_input_conditions_digest":digest,
            "dietary_review_pin":review_pin,"maker_policy_pin":policy_pin})
        || policy["accepts"]["intrinsic_conditions_apply_even_None"]!=true
        || policy["accepts"]["dietary_review_id"]!=review_id || policy["accepts"]["fixed_units"]!=2
        || policy["accepts"]["first_handling_offset_seconds"]!=-5400 {
        return Err(fail(ErrorCode::DigestMismatch,"/maker/intrinsic/custody"));
    }
    if table["product_and_operational_compliance_observed"]!=false
        || review["actual_products_and_processing_observed"]!=false
        || review["trademark_GMO_animal_testing_allergen_or_other_diet_supported"]!=false {
        return Err(fail(ErrorCode::AuthorityDenied,"/maker/intrinsic/product_authority"));
    }
    Ok(())
}
fn validate_larger_porridge(record: &ReviewedPacketRecord, recipe: &RecipeRevision,
    reference: &RecipeRef) -> Result<(), DomainError> {
    let documents=record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/larger/documents"))?;
    if documents.len()!=11 {return Err(fail(ErrorCode::MissingReference,"/maker/larger/documents"));}
    let rf=serde_json::to_value(reference).unwrap();
    for (document,suffix) in documents.iter().zip(["33","34","35","36","37","38","39","3a","3b","3c","3d"]) {
        if document["evidence_id"]!=format!("ev_800000000000000000000000000000{suffix}")
            || document["revision"]!="1" || document["actor"]!="automated" || document["recipe_reference"]!=rf {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/larger/document_closure"));
        }
    }
    let ingredients=recipe.ingredients.iter().map(|ingredient|serde_json::json!({
        "ingredient_id":ingredient.ingredient_id,"quantity":ingredient.quantity})).collect::<Vec<_>>();
    let expected=serde_json::json!([
        {"ingredient_id":"ig_80000000000000000000000000000001","quantity":{"amount":{"numerator":"135","denominator":"2"},"unit":"g","form":"dry"}},
        {"ingredient_id":"ig_80000000000000000000000000000002","quantity":{"amount":{"numerator":"270","denominator":"1"},"unit":"ml","form":"raw"}},
        {"ingredient_id":"ig_80000000000000000000000000000003","quantity":{"amount":{"numerator":"270","denominator":"1"},"unit":"ml","form":"raw"}}]);
    let one=PositiveRational::new(1,1).unwrap();
    if serde_json::json!(ingredients)!=expected || documents[1]["ingredients"]!=expected
        || record.ingredient_registry["entries"]!=document(REGISTRY)?["entries"]
        || record.source_captures!=document(CAPTURES)?
        || recipe.r#yield.servings!=one || recipe.r#yield.components.len()!=1
        || recipe.r#yield.components[0].component_id.0!="co_80000000000000000000000000000007"
        || recipe.r#yield.components[0].quantity!=(Quantity{amount:one,unit:Unit::Each,form:IngredientForm::Cooked})
        || recipe.scaling!=(ScalingPolicy::Linear{minimum:one,maximum:one})
        || recipe.portion_presets.len()!=1 || recipe.portion_presets[0].preset!=PortionPresetName::Larger
        || recipe.portion_presets[0].age_band!=AgeBand::Adult || recipe.portion_presets[0].meal_type!=MealType::Breakfast
        || recipe.portion_presets[0].servings!=one
        || recipe.portion_presets[0].evidence_ids.0!=vec![EvidenceId("ev_80000000000000000000000000000034".into()),EvidenceId("ev_8000000000000000000000000000003d".into())] {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/larger/fixed_unit"));
    }
    let method=&documents[4];let policy=&documents[10];
    if recipe.equipment.0!=vec![EquipmentCode::Hob] || recipe.prep.len()!=1 || recipe.steps.len()!=1
        || recipe.active_minutes!=20 || recipe.elapsed_minutes!=20
        || recipe.prep[0].active_minutes!=20 || recipe.prep[0].elapsed_minutes!=20
        || recipe.prep[0].earliest_offset_seconds!=-1200 || recipe.prep[0].latest_offset_seconds!=-1200
        || recipe.prep[0].can_attach_to_prior_session || !recipe.prep[0].depends_on.is_empty()
        || method["method_production"]!=serde_json::json!({"component_id":"co_80000000000000000000000000000007","kind":"cooked","completion_prep_id":1,"prep_ids":[1]})
        || method["phase_resources"]!=serde_json::json!([{"prep_id":1,"resources":["hob"]}])
        || method["timing_measured"]!=false || method["thermal_completion_guaranteed"]!=false
        || policy["accepts"]["native_multiplier"]!=serde_json::json!(one)
        || policy["accepts"]["all_applicable_portion_adjustments"]!=serde_json::json!(one)
        || policy["accepts"]["fixed_larger_unit"]["ingredients"]!=expected
        || policy["accepts"]["attention_reservation"]["minutes"]!=20
        || policy["accepts"]["attention_reservation"]["thermal_completion_guaranteed"]!=false
        || policy["limits"]["human_cook_test"]!=false || policy["limits"]["human_calibration"]!=false
        || policy["limits"]["hard_dietary_requirements_supported"]!=false {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/larger/method_scope"));
    }
    if recipe.reviews.len()!=3 || recipe.reviews.iter().any(|review|review.actor!=ReviewActor::Automated||review.kind==ReviewKind::CookTest)
        || recipe.dietary_evidence.len()!=5 || recipe.dietary_evidence.iter().any(|row|row.verdict==DietaryVerdict::Supported)
        || documents[2]["product_labels_observed"]!=false || documents[2]["cross_contact_observed"]!=false
        || documents[0]["fresh_network_retrieval"]!=false
        || recipe.image.0.is_some() || recipe.nutrition.0.is_some() || !recipe.storage.is_empty()
        || !recipe.reused_inputs.is_empty() || !recipe.extras.is_empty() {
        return Err(fail(ErrorCode::AuthorityDenied,"/maker/larger/unsupported_claim"));
    }
    for (row,document) in recipe.dietary_evidence.iter().zip(&documents[5..10]) {
        if serde_json::to_value(&row.evidence_id).unwrap()!=document["evidence_id"]
            || serde_json::to_value(&row.requirement).unwrap()!=document["requirement"]
            || serde_json::to_value(&row.verdict).unwrap()!=document["verdict"]
            || row.review_id.0!="ev_80000000000000000000000000000035" || document["review_id"]!=row.review_id.0 {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/larger/dietary_closure"));
        }
    }
    Ok(())
}
#[derive(Deserialize,Serialize,JsonSchema)]
#[serde(deny_unknown_fields)]
struct PhaseEquipmentEntry { prep_id:u16, equipment:BoundedVec<EquipmentCode,0,32> }
fn pair_entry(compiled:&CompiledPacket)->bool {
    compiled.scope.recipe_id==THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id
        || compiled.scope.recipe_id==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id
}
fn unique_document<'a>(documents:&'a [Value],id:&str)->Result<&'a Value,DomainError>{
    let mut found=documents.iter().filter(|document|document["evidence_id"]==id);
    let value=found.next().ok_or_else(||fail(ErrorCode::MissingReference,"/maker/document/id"))?;
    if found.next().is_some(){return Err(fail(ErrorCode::DuplicateKey,"/maker/document/id"));}Ok(value)
}
fn phase_map(record:&ReviewedPacketRecord,recipe:&RecipeRevision,reference:&RecipeRef,
    resource:&Value,policy:&Value,basis:&EvidenceBasis)->Result<Vec<(u16,Vec<EquipmentCode>)>,DomainError>{
    let map=&record.packet["phase_resource_map"];let rf=serde_json::json!(reference);let resource_digest=canonical_digest(resource)?;
    let resource_id=map["resource_evidence_id"].as_str().ok_or_else(||fail(ErrorCode::MissingReference,"/maker/phase_resources/evidence_id"))?;
    if map["format"]!="mstrchf-exact-reviewed-phase-resources" || map["version"]!=1 || map["recipe_reference"]!=rf
        || resource["recipe_reference"]!=rf || resource["evidence_id"]!=resource_id
        || !basis.admitted.iter().any(|pin|pin.evidence_id.0==resource_id&&pin.digest==resource_digest)
        || resource["full_method"]!=serde_json::json!(recipe.steps) {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/phase_resources/binding"));
    }
    let digest=serde_json::json!(canonical_digest(map)?);
    if record.packet["phase_resource_map_digest"]!=digest || resource["phase_resource_map_digest"]!=digest
        || policy["accepted_phase_resource_map_digest"]!=digest || resource["phase_resources"]!=map["entries"] {
        return Err(fail(ErrorCode::DigestMismatch,"/maker/phase_resources/custody"));
    }
    let rows:BoundedVec<PhaseEquipmentEntry,1,256>=parse_document(&canonical_bytes(&map["entries"])?)?;
    if rows.len()!=recipe.prep.len(){return Err(fail(ErrorCode::MissingReference,"/maker/phase_resources/coverage"));}
    let mut result=Vec::new();
    for (index,row) in rows.iter().enumerate(){
        if index>0&&rows[index-1].prep_id>=row.prep_id {return Err(fail(ErrorCode::DuplicateKey,"/maker/phase_resources/prep_id"));}
        if !recipe.prep.iter().any(|prep|prep.prep_id==row.prep_id) {return Err(fail(ErrorCode::ForeignReference,"/maker/phase_resources/prep_id"));}
        // Equipment lists use canonical wire order, as in contracts and native
        // food validation. Enum declaration order differs (Hob precedes Fridge).
        if row.equipment.windows(2).any(|pair|serde_json::json!(pair[0]).as_str()>=serde_json::json!(pair[1]).as_str())
            || row.equipment.iter().any(|equipment|!recipe.equipment.contains(equipment)) {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/phase_resources/equipment"));
        }
        result.push((row.prep_id,row.equipment.0.clone()));
    }
    Ok(result)
}
fn validate_chicken_pair(record:&ReviewedPacketRecord,recipe:&RecipeRevision,reference:&RecipeRef,
    compiled:&CompiledPacket,policy:&Value,basis:&EvidenceBasis)->Result<Vec<(u16,Vec<EquipmentCode>)>,DomainError>{
    let extra=compiled.scope.recipe_id==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id;
    let docs=record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/pair/documents"))?;
    let suffixes:&[&str]=if extra{&["3e","3f","40","41","42","43","44","45","4d"]}else{&["46","47","48","49","4a","4b","4c"]};
    let rf=serde_json::json!(reference);
    for (document,suffix) in docs.iter().zip(suffixes){
        if document["evidence_id"]!=format!("ev_800000000000000000000000000000{suffix}") || document["revision"]!="1"
            || document["actor"]!="automated" || document["recipe_reference"]!=rf {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/pair/document_closure"));
        }
    }
    let resource=unique_document(docs,compiled.resource_id)?;let method=unique_document(docs,compiled.method_id)?;
    let diet_id=if extra{"ev_80000000000000000000000000000044"}else{"ev_8000000000000000000000000000004b"};
    let diet=unique_document(docs,diet_id)?;let table=&record.packet["intrinsic_input_conditions"];
    let table_digest=canonical_digest(table)?;let map_digest=canonical_digest(&record.packet["phase_resource_map"])?;
    let expected_table=if extra{"9947369c9ceb7640855ce2d0a5483096588597171b9f526489b730b9ab720da0"}else{"68d632a4173ce72feec8771e6d7ceafd1c7d7b33523ea12912a57893321cd0a1"};
    let expected_map=if extra{"b3eeed521d11bfa8d4906cbf3b9fd2eeda7d06296216402b0da8848317388ef5"}else{"8be352df60af8b7a9297f7cbfd176e4c93b40f9509ae18ef7a4c5834fc578837"};
    binding(table,expected_table,"/maker/pair/intrinsic")?;binding(&record.packet["phase_resource_map"],expected_map,"/maker/pair/phase_map")?;
    let dietary_pin=serde_json::json!({"evidence_id":diet_id,"revision":"1","digest":canonical_digest(diet)?});
    let policy_pin=serde_json::json!({"evidence_id":compiled.policy_id,"revision":"1","digest":canonical_digest(policy)?});
    let ingredients=serde_json::json!(recipe.ingredients.iter().map(|ingredient|&ingredient.ingredient_id).collect::<Vec<_>>());
    let prep_ids=serde_json::json!(recipe.prep.iter().map(|prep|prep.prep_id).collect::<Vec<_>>());
    let step_ids=serde_json::json!(recipe.steps.iter().map(|step|step.step_id).collect::<Vec<_>>());
    if table["format"]!="mstrchf-recipe-intrinsic-input-conditions" || table["version"]!=1
        || table["recipe_reference"]!=rf || table["dietary_review_id"]!=diet_id || table["maker_policy_id"]!=compiled.policy_id
        || table["full_method_step_ids"]!=step_ids || table["full_method_prep_ids"]!=prep_ids
        || table["applies_even_when_all_consumers_confirmed_none"]!=true || table["product_and_operational_compliance_observed"]!=false
        || diet["coverage"]["ingredient_ids"]!=ingredients || diet["coverage"]["step_ids"]!=step_ids || diet["coverage"]["prep_ids"]!=prep_ids
        || diet["full_method"]!=serde_json::json!(recipe.steps) || method["full_method"]!=serde_json::json!(recipe.steps)
        || method["phases"]!=serde_json::json!(recipe.prep) || method["production_method_prep_ids"]!=prep_ids
        || diet["intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest) || record.packet["intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest)
        || policy["accepted_intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest) || policy["accepted_dietary_review_pin"]!=dietary_pin
        || policy["accepted_source_captures_digest"]!=record.packet["source_captures_digest"]
        || record.packet["intrinsic_input_custody"]!=serde_json::json!({"recipe_reference":reference,"intrinsic_input_conditions_digest":table_digest,
            "phase_resource_map_digest":map_digest,"dietary_review_pin":dietary_pin,"maker_policy_pin":policy_pin}) {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/pair/closure"));
    }
    let entries=table["entries"].as_array().ok_or_else(||fail(ErrorCode::MissingReference,"/maker/pair/input_entries"))?;
    if entries.len()!=recipe.ingredients.len(){return Err(fail(ErrorCode::MissingReference,"/maker/pair/input_entries"));}
    for (row,ingredient) in entries.iter().zip(recipe.ingredients.iter()){
        let quantity=ingredient.quantity.0.as_ref().ok_or_else(||fail(ErrorCode::MissingReference,"/maker/pair/quantity"))?;
        if row["ingredient_id"]!=serde_json::json!(ingredient.ingredient_id)||row["recipe_input_name"]!=serde_json::json!(ingredient.name)
            || row["form"]!=serde_json::json!(quantity.form)||row["unit"]!=serde_json::json!(quantity.unit){return Err(fail(ErrorCode::IdentityMismatch,"/maker/pair/input"));}
    }
    let phase_equipment=phase_map(record,recipe,reference,resource,policy,basis)?;
    if extra {
        if !recipe.source.url.0.is_none() || recipe.source.origin!=RecipeSourceOrigin::UserAuthored || compiled.scope.source_url!=""
            || recipe.text_rights.distribution!=RightsDistribution::PersonalOnly || recipe.text_rights.licence_url.0.is_some()
            || docs[0]["human_authorship_claimed"]!=false || docs[0]["generated_by"]!="Codex under owner commission"
            || recipe.ingredients.len()!=1 || recipe.ingredients[0].ingredient_id.0!="ig_80000000000000000000000000000010"
            || recipe.ingredients[0].quantity.0.as_ref()!=Some(&Quantity{amount:PositiveRational::new(50,1)?,unit:Unit::G,form:IngredientForm::Cooked})
            || recipe.extras.len()!=0 || recipe.prep.len()!=1 || phase_equipment!=vec![(1,vec![])]
            || recipe.equipment.0!=vec![EquipmentCode::Fridge] || !recipe.portability.cold_storage_required
            || docs[7]["evidence_id"]!=compiled.policy_id || docs[8]["evidence_id"]!="ev_8000000000000000000000000000004d"
            || recipe.dietary_evidence.len()!=2 || recipe.dietary_evidence.iter().any(|row|row.verdict!=DietaryVerdict::Conflicting)
            || record.ingredient_registry["entries"].as_array().map_or(0,Vec::len)!=1 {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/pair/extra"));
        }
    } else {
        let child=serde_json::json!({"recipe_id":CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id,"revision":"1","digest":CHILLED_CHICKEN_EXTRA_SCOPE.recipe_digest});
        let original_registry=document(COMPILED[4].registry)?;let registry=record.ingredient_registry["entries"].as_array().ok_or_else(||fail(ErrorCode::MissingReference,"/maker/pair/registry"))?;
        let child_registry=document(COMPILED[7].registry)?;
        if recipe.ingredients.len()!=9 || recipe.extras.len()!=1 || serde_json::json!(recipe.extras[0].recipe)!=child
            || recipe.extras[0].extra_id.0!="ex_80000000000000000000000000000002"
            || recipe.extras[0].supported_with.0!=vec![ComponentId("co_80000000000000000000000000000008".into())]
            || record.packet["declared_extra_dependency"]["admission_order"]!="extra_first_then_base"
            || record.packet["declared_extra_dependency"]["extra_reference"]!=child || policy["accepted_extra_reference"]!=child
            || record.packet["declared_extra_dependency"]["extra_packet_digest"]!=CHILLED_CHICKEN_EXTRA_SCOPE.packet_canonical_digest
            || record.packet["declared_extra_dependency"]["base_without_selected_extra_still_requires_dependency"]!=true
            || policy["accepted_extra_packet_digest"]!=CHILLED_CHICKEN_EXTRA_SCOPE.packet_canonical_digest
            || table["rule"]!=VEGETARIAN_INPUT_RULE || diet["coverage"]["all_ingredient_input_conditions"]!=table["entries"]
            || diet["optional_extra_in_base_composition"]!=false || diet["animal_extra_recipient_combined_meal_vegetarian"]!=false
            || recipe.dietary_evidence.len()!=1 || recipe.dietary_evidence[0].verdict!=DietaryVerdict::Supported
            || recipe.dietary_evidence[0].requirement!=(HardRequirement::Diet{code:DietCode::Vegetarian})
            || phase_equipment!=vec![(1,vec![EquipmentCode::Fridge]),(2,vec![EquipmentCode::Fridge])]
            || registry.len()!=10 || registry[..9]!=original_registry["entries"].as_array().unwrap()[..]
            || registry[9]!=child_registry["entries"][0] || record.source_captures!=document(COMPILED[4].captures)? {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/pair/base"));
        }
    }
    Ok(phase_equipment)
}
fn load(compiled: &CompiledPacket) -> Result<TrustedPacket, DomainError> {
    let record = ReviewedPacketRecord {
        packet: document(compiled.packet)?, evidence_documents: document(compiled.documents)?,
        ingredient_registry: document(compiled.registry)?, source_captures: document(compiled.captures)?,
    };
    binding(&record.packet, compiled.scope.packet_canonical_digest, "/maker/packet")?;
    binding(&record.evidence_documents, compiled.documents_digest, "/maker/evidence_documents")?;
    binding(&record.ingredient_registry, compiled.registry_digest, "/maker/ingredient_registry")?;
    binding(&record.source_captures, compiled.captures_digest, "/maker/source_captures")?;
    let recipe: RecipeRevision = parse_document(&serde_json::to_vec(&record.packet["maker_recipe_revision"]).map_err(|_|fail(ErrorCode::InvalidValue,"/maker/recipe"))?)?;
    let reference: RecipeRef = parse_document(&canonical_bytes(&record.packet["recipe_reference"])?)?;
    if recipe.recipe_id.0 != compiled.scope.recipe_id
        || recipe.revision.0 != compiled.scope.recipe_revision
        || recipe_digest(&recipe)? != reference.digest
        || reference.digest.0 != compiled.scope.recipe_digest
        || reference.recipe_id != recipe.recipe_id || reference.revision != recipe.revision
    { return Err(fail(ErrorCode::DigestMismatch, "/maker/recipe_reference")); }
    let documents = record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/evidence_documents"))?;
    let pins = record.packet["maker_evidence_pins"].as_array().ok_or_else(||fail(ErrorCode::InvalidValue,"/maker/evidence_pins"))?;
    if documents.len() != compiled.document_count || pins.len() != compiled.document_count { return Err(fail(ErrorCode::MissingReference,"/maker/evidence_pins")); }
    let mut admitted = Vec::new();
    let mut not_before: Option<Instant> = None;
    for (document, pin) in documents.iter().zip(pins) {
        let observed_at: Instant = parse_document(&canonical_bytes(&document["at"])?)?;
        if not_before.as_ref().is_none_or(|old| old < &observed_at) { not_before = Some(observed_at); }
        let pin: PinDocument = parse_document(&canonical_bytes(pin)?)?;
        if canonical_digest(document)? != pin.digest || document["evidence_id"] != serde_json::to_value(&pin.evidence_id).unwrap()
            || document["revision"] != serde_json::to_value(pin.revision).unwrap()
        { return Err(fail(ErrorCode::DigestMismatch,"/maker/evidence_pin")); }
        admitted.push(EvidencePin {evidence_id:pin.evidence_id,revision:pin.revision,digest:pin.digest});
    }
    for review in recipe.reviews.iter() {
        // The recipe body passed full shape admission, but explicitly revalidate
        // every timestamp before relying on canonical Instant ordering.
        let observed_at: Instant = parse_document(&canonical_bytes(&review.at)?)?;
        if not_before.as_ref().is_none_or(|old| old < &observed_at) { not_before = Some(observed_at); }
    }
    let not_before = not_before.ok_or_else(||fail(ErrorCode::MissingReference,"/maker/not_before"))?;
    // The new extra has a later composition fact after its policy. Select
    // new-pair policies by ID, preserve old rules, and never infer authorisation
    // from the last transported document or the frozen text's status alone.
    let policy = if pair_entry(compiled)||courgette::entry(compiled)||portable::entry(compiled)||gf_chicken::entry(compiled)||chilli::entry(compiled)||overnight::entry(compiled){unique_document(documents,compiled.policy_id)?}else{documents.last().unwrap()};
    // The Smaller packet's recording predates root's exact-body adoption here.
    // Preserve its historical text; only this literal compiled body is trusted.
    let expected_status=if compiled.scope.recipe_id==THREE_BEAN_SMALLER_SCOPE.recipe_id {"maker_direction_adopted_exact_formal_body_review_pending"}
        else if pair_entry(compiled){"accepted_authoring_specification_pending_independent_formal_review_and_runtime_integration"}
        else{"accepted_for_compiled_maker_registry_limited_scope"};
    if policy["recipe_reference"] != record.packet["recipe_reference"]
        || policy["accepted_ingredient_registry_digest"] != record.packet["ingredient_registry_digest"]
        || policy["accepts"]["operational_scope"] != record.packet["operational_scope"]
        || record.packet["operational_scope"]["scope_id"] != compiled.scope.scope_id
        || policy["status"] != expected_status
    { return Err(fail(ErrorCode::AuthorityDenied,"/maker/scope")); }
    binding(policy,compiled.policy_digest,"/maker/root_policy")?;
    if policy["evidence_id"] != compiled.policy_id { return Err(fail(ErrorCode::IdentityMismatch,"/maker/root_policy/id")); }
    let relevant: BoundedVec<IngredientId,0,128> = match record.packet.get("maker_relevant_essential_ingredient_ids") {
        Some(value) => parse_document(&canonical_bytes(value)?)?, None => BoundedVec(vec![]),
    };
    if relevant.windows(2).any(|pair|pair[0]>=pair[1])
        || relevant.iter().any(|id|!recipe.ingredients.iter().any(|ingredient|&ingredient.ingredient_id==id))
        || (!relevant.is_empty() && policy["accepts"]["relevant_essentials"]["ingredient_ids"] != serde_json::to_value(&relevant).unwrap())
    { return Err(fail(ErrorCode::IdentityMismatch,"/maker/relevant_essentials")); }
    if compiled.scope.recipe_id==THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE.recipe_id {
        validate_vegetarian_inputs(&record,&recipe,&reference)?;
    }
    if compiled.scope.recipe_id==PORRIDGE_LARGER_SCOPE.recipe_id {
        validate_larger_porridge(&record,&recipe,&reference)?;
    }
    let basis = EvidenceBasis::new(reference.clone(), admitted, vec![])?;
    for id in [compiled.method_id,compiled.resource_id] {
        if !basis.admitted.iter().any(|pin|pin.evidence_id.0==id) {return Err(fail(ErrorCode::MissingReference,"/maker/food_evidence"));}
    }
    let (classes,phase_equipment)=if overnight::entry(compiled) {
        (ComponentClassView::new(reference.clone(),vec![])?,
            Some(overnight::evidence(&record,&recipe,&reference,compiled,policy,&basis)?))
    } else if chilli::entry(compiled) {
        let (classes,equipment)=chilli::evidence(&record,&recipe,&reference,compiled,policy,&basis)?;
        (classes,Some(equipment))
    } else if gf_chicken::entry(compiled) {
        (ComponentClassView::new(reference.clone(),vec![])?,
            Some(gf_chicken::evidence(&record,&recipe,&reference,compiled,policy,&basis)?))
    } else if portable::entry(compiled) {
        let (classes,equipment)=portable::evidence(&record,&recipe,&reference,compiled,policy,&basis)?;
        (classes,Some(equipment))
    } else if courgette::entry(compiled) {
        let (classes,equipment)=courgette::evidence(&record,&recipe,&reference,compiled,policy,&basis)?;
        (classes,Some(equipment))
    } else {
        (ComponentClassView::new(reference.clone(), vec![])?,
            if pair_entry(compiled){Some(validate_chicken_pair(&record,&recipe,&reference,compiled,policy,&basis)?)}else{None})
    };
    assess_revision(&recipe, &basis, &classes)?;
    Ok(TrustedPacket {record,reference:ReviewedRecipePacketRef {recipe:reference,packet_digest:Digest(compiled.scope.packet_canonical_digest.into())},recipe,basis,classes,not_before,
        scope:compiled.scope,policy_id:EvidenceId(compiled.policy_id.into()),
        method_id:EvidenceId(compiled.method_id.into()),resource_id:EvidenceId(compiled.resource_id.into()),relevant_essentials:relevant.0,phase_equipment})
}
fn registry() -> Result<&'static Vec<TrustedPacket>, DomainError> {
    static PACKETS: OnceLock<Result<Vec<TrustedPacket>, DomainError>> = OnceLock::new();
    PACKETS.get_or_init(||COMPILED.iter().map(load).collect()).as_ref().map_err(Clone::clone)
}
pub fn porridge_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[0].reference().clone())
}
pub fn aglio_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[1].reference().clone())
}
pub fn three_bean_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[2].reference().clone())
}
pub fn three_bean_two_unit_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[3].reference().clone())
}
pub fn three_bean_two_unit_vegetarian_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[4].reference().clone())
}
pub fn larger_porridge_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[5].reference().clone())
}
pub fn three_bean_two_unit_chicken_extra_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[6].reference().clone())
}
pub fn chilled_chicken_extra_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[7].reference().clone())
}
pub fn three_bean_smaller_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[8].reference().clone())
}
pub fn courgette_hot_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[9].reference().clone())
}
pub fn courgette_cold_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[10].reference().clone())
}
pub fn portable_hot_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[11].reference().clone())
}
pub fn portable_cold_packet_ref() -> Result<ReviewedRecipePacketRef, DomainError> {
    Ok(registry()?[12].reference().clone())
}
pub fn three_bean_gf_chicken_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/gf_pair/base"))
}
pub fn chilled_gf_chicken_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==crate::personal_scope::CHILLED_GF_CHICKEN_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/gf_pair/extra"))
}
pub fn chilli_batch_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==CHILLI_BATCH_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/chilli/packet"))
}
pub fn chilli_wraps_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==CHILLI_WRAPS_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/chilli/packet"))
}
pub fn chilli_potato_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==CHILLI_POTATO_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/chilli/packet"))
}
pub fn available_packet_refs() -> Result<Vec<ReviewedRecipePacketRef>, DomainError> {
    Ok(registry()?.iter().map(|packet|packet.reference().clone()).collect())
}
pub fn overnight_oats_packet_ref() -> Result<ReviewedRecipePacketRef,DomainError> {
    registry()?.iter().find(|packet|packet.scope.recipe_id==OVERNIGHT_OATS_SCOPE.recipe_id)
        .map(|packet|packet.reference().clone()).ok_or_else(||fail(ErrorCode::MissingReference,"/maker/overnight/packet"))
}
pub fn known_packet(reference: &ReviewedRecipePacketRef) -> Result<&'static TrustedPacket, DomainError> {
    reference.validate_shape().map_err(|_|fail(ErrorCode::InvalidValue,"/maker/reference"))?;
    let packet = registry()?.iter().find(|packet|reference.recipe.recipe_id==packet.reference.recipe.recipe_id&&reference.recipe.revision==packet.reference.recipe.revision)
        .ok_or_else(||fail(ErrorCode::MissingReference,"/maker/reference"))?;
    if reference != packet.reference() { return Err(fail(ErrorCode::DigestMismatch,"/maker/reference")); }
    Ok(packet)
}
/// Exact record matching is custody validation against a compiled registry.
/// Values supplied by a caller never construct an EvidenceBasis.
pub fn known_record(record: &ReviewedPacketRecord) -> Result<&'static TrustedPacket, DomainError> {
    if canonical_bytes(record)?.len() > 1024 * 1024 { return Err(fail(ErrorCode::LimitExceeded,"/maker/record")); }
    registry()?.iter().find(|packet|record==packet.record()).ok_or_else(||fail(ErrorCode::DigestMismatch,"/maker/record"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_compiled_record_passes_document_shape_and_exact_custody() {
        for compiled in &COMPILED {
            for (name,bytes) in [("packet",compiled.packet),("documents",compiled.documents),
                ("registry",compiled.registry),("captures",compiled.captures)] {
                document(bytes).unwrap_or_else(|failure|panic!("{} {name}: {failure:?}",compiled.scope.recipe_id));
            }
            load(compiled).unwrap_or_else(|failure|panic!("{} custody: {failure:?}",compiled.scope.recipe_id));
        }
    }

    #[test]
    fn exact_phase_accessor_keeps_old_maps_and_refuses_unknown_or_incomplete_new_maps(){
        for reference in available_packet_refs().unwrap().into_iter().take(6){let packet=known_packet(&reference).unwrap();for prep in packet.recipe().prep.iter(){assert_eq!(packet.phase_equipment(prep.prep_id).unwrap(),packet.recipe().equipment.0.as_slice());}assert!(packet.phase_equipment(65535).is_err());}
        let packet=known_packet(&chilled_chicken_extra_packet_ref().unwrap()).unwrap();assert!(packet.phase_equipment(1).unwrap().is_empty());assert!(packet.phase_equipment(2).is_err());
        let base=known_packet(&three_bean_two_unit_chicken_extra_packet_ref().unwrap()).unwrap();assert_eq!(base.phase_equipment(1).unwrap(),&[EquipmentCode::Fridge]);assert_eq!(base.phase_equipment(2).unwrap(),&[EquipmentCode::Fridge]);
        for mutate in [0,1,2,3,4]{
            let mut record=base.record().clone();
            match mutate{
                0=>record.packet["phase_resource_map"]["entries"]=json!([]),
                1=>record.packet["phase_resource_map"]["entries"][1]["prep_id"]=json!(1),
                2=>record.packet["phase_resource_map"]["entries"][1]["prep_id"]=json!(3),
                3=>record.packet["phase_resource_map"]["recipe_reference"]["digest"]=json!("0".repeat(64)),
                _=>record.packet["phase_resource_map"]["entries"][0]["equipment"]=json!(["fridge","fridge"]),
            }
            let mut resource=unique_document(record.evidence_documents.as_array().unwrap(),"ev_80000000000000000000000000000049").unwrap().clone();
            let mut policy=unique_document(record.evidence_documents.as_array().unwrap(),"ev_8000000000000000000000000000004c").unwrap().clone();
            let digest=json!(canonical_digest(&record.packet["phase_resource_map"]).unwrap());record.packet["phase_resource_map_digest"]=digest.clone();resource["phase_resource_map_digest"]=digest.clone();resource["phase_resources"]=record.packet["phase_resource_map"]["entries"].clone();policy["accepted_phase_resource_map_digest"]=digest;
            // Self-consistent test-only pins cannot rescue missing, duplicate,
            // foreign or malformed phase closure even at this private helper.
            let mut basis=base.basis().clone();basis.admitted.iter_mut().find(|pin|pin.evidence_id.0=="ev_80000000000000000000000000000049").unwrap().digest=canonical_digest(&resource).unwrap();
            assert!(phase_map(&record,base.recipe(),&base.reference().recipe,&resource,&policy,&basis).is_err());
        }
        let mut missing=packet.basis().clone();missing.admitted.retain(|pin|pin.evidence_id.0!="ev_80000000000000000000000000000042");
        let documents=packet.record().evidence_documents.as_array().unwrap();assert!(phase_map(packet.record(),packet.recipe(),&packet.reference().recipe,unique_document(documents,"ev_80000000000000000000000000000042").unwrap(),unique_document(documents,"ev_80000000000000000000000000000045").unwrap(),&missing).is_err());
    }

    #[test]
    fn phase_resources_use_wire_order_and_refuse_duplicate_reversed_or_foreign_equipment() {
        let packet=known_packet(&chilli_potato_packet_ref().unwrap()).unwrap();
        assert_eq!(packet.phase_equipment(4).unwrap(),&[EquipmentCode::Fridge,EquipmentCode::Hob]);
        let resource_id=&packet.phase_resource_evidence_id().0;
        let policy_id=&packet.root_policy_evidence_id().0;
        for (equipment,accepted) in [
            (json!(["fridge","hob"]),true),
            (json!(["fridge","fridge"]),false),
            (json!(["hob","fridge"]),false),
            (json!(["fridge","kettle"]),false),
        ] {
            let mut record=packet.record().clone();
            record.packet["phase_resource_map"]["entries"][3]["equipment"]=equipment.clone();
            let documents=record.evidence_documents.as_array().unwrap();
            let mut resource=unique_document(documents,resource_id).unwrap().clone();
            let mut policy=unique_document(documents,policy_id).unwrap().clone();
            let digest=json!(canonical_digest(&record.packet["phase_resource_map"]).unwrap());
            record.packet["phase_resource_map_digest"]=digest.clone();
            resource["phase_resource_map_digest"]=digest.clone();
            resource["phase_resources"]=record.packet["phase_resource_map"]["entries"].clone();
            policy["accepted_phase_resource_map_digest"]=digest;
            // Rebind test-only custody so rejection tests the resource list,
            // rather than merely detecting a changed document digest.
            let mut basis=packet.basis().clone();
            basis.admitted.iter_mut().find(|pin|&pin.evidence_id.0==resource_id).unwrap().digest=canonical_digest(&resource).unwrap();
            let result=phase_map(&record,packet.recipe(),&packet.reference().recipe,&resource,&policy,&basis);
            if accepted {
                assert_eq!(result.unwrap()[3],(4,vec![EquipmentCode::Fridge,EquipmentCode::Hob]));
            } else {
                assert!(result.is_err(),"Invalid phase equipment accepted: {equipment}");
            }
        }
    }

    #[test]
    fn three_bean_exact_compiled_inputs_and_latest_clock_are_bound(){
        let reference=three_bean_packet_ref().unwrap();let packet=known_packet(&reference).unwrap();
        assert_eq!(reference.recipe.recipe_id.0,THREE_BEAN_PERSONAL_SCOPE.recipe_id);
        assert_eq!(reference.recipe.digest.0,"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377");
        assert_eq!(reference.packet_digest.0,"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac");
        assert_eq!(packet.not_before(),&Instant("2026-10-12T21:02:09Z".into()));
        assert_eq!(packet.root_policy_evidence_id().0,"ev_80000000000000000000000000000025");
        assert_eq!(packet.basis().admitted.len(),6);assert!(packet.classes().components.is_empty());
        assert!(packet.relevant_essential_ingredient_ids().is_empty());
        let recipe=packet.recipe();assert_eq!(recipe.equipment.0,vec![EquipmentCode::Fridge]);
        assert_eq!(recipe.ingredients.len(),9);assert_eq!(recipe.prep.len(),2);
        assert_eq!(recipe.prep[0].active_minutes,15);assert_eq!(recipe.prep[1].active_minutes,0);
        assert_eq!(recipe.prep[1].elapsed_minutes,60);
        assert!(matches!(&recipe.components[0].production.0,Some(ComponentProductionSpec::Unheated{first_handling_prep_id:1})));
        assert!(recipe.storage.is_empty()&&recipe.reused_inputs.is_empty()&&recipe.dietary_evidence.is_empty());
        assert!(recipe.image.0.is_none()&&recipe.nutrition.0.is_none());
        assert!(recipe.reviews.iter().all(|review|review.actor==ReviewActor::Automated&&review.kind!=ReviewKind::CookTest));
        assert!(!assess_revision(recipe,packet.basis(),packet.classes()).unwrap().assessment().capabilities.leftovers);
        assert_eq!(known_record(packet.record()).unwrap().reference(),&reference);
    }
    #[test]
    fn three_bean_self_resigned_transport_cannot_manufacture_portable_authority(){
        let original=known_packet(&three_bean_packet_ref().unwrap()).unwrap();let mut forged=original.record().clone();
        forged.packet["maker_recipe_revision"]["equipment"]=json!([]);
        forged.packet["operational_scope"]["location"]=json!("on_the_move");
        forged.packet["operational_scope"]["home_cold_handling"]["portable_route_qualified"]=json!(true);
        let recipe:RecipeRevision=parse_document(&serde_json::to_vec(&forged.packet["maker_recipe_revision"]).unwrap()).unwrap();
        let recipe_ref=RecipeRef{recipe_id:recipe.recipe_id.clone(),revision:recipe.revision,digest:recipe_digest(&recipe).unwrap()};
        forged.packet["recipe_reference"]=serde_json::to_value(&recipe_ref).unwrap();
        let scope=forged.packet["operational_scope"].clone();let policy=forged.evidence_documents.as_array_mut().unwrap().last_mut().unwrap();
        policy["recipe_reference"]=serde_json::to_value(&recipe_ref).unwrap();policy["accepts"]["operational_scope"]=scope;
        policy["limits"]["portable"]=json!(true);
        let pins=forged.evidence_documents.as_array().unwrap().iter().map(|doc|json!({"evidence_id":doc["evidence_id"],"revision":doc["revision"],"digest":canonical_digest(doc).unwrap()})).collect::<Vec<_>>();
        forged.packet["maker_evidence_pins"]=json!(pins);forged.packet["evidence_documents_digest"]=json!(canonical_digest(&forged.evidence_documents).unwrap());
        let reference=ReviewedRecipePacketRef{recipe:recipe_ref,packet_digest:canonical_digest(&forged.packet).unwrap()};
        assert_eq!(known_record(&forged).unwrap_err().code,ErrorCode::DigestMismatch);
        assert_eq!(known_packet(&reference).unwrap_err().code,ErrorCode::DigestMismatch);
        // Original custody still resolves, but neither input contains native
        // EvidenceBasis authority or an observed product/fridge/readiness fact.
        assert_eq!(known_record(original.record()).unwrap().reference(),original.reference());
    }
    #[test]
    fn three_bean_registration_preserves_old_packet_prefix_bodies_and_pins(){
        let refs=available_packet_refs().unwrap();assert_eq!(refs.len(),19);
        assert_eq!(refs[0],porridge_packet_ref().unwrap());assert_eq!(refs[1],aglio_packet_ref().unwrap());assert_eq!(refs[2],three_bean_packet_ref().unwrap());
        assert_eq!(refs[3],three_bean_two_unit_packet_ref().unwrap());
        for (index,digest) in [(0,"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726"),(1,"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee"),(2,"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac")]{
            let packet=known_packet(&refs[index]).unwrap();assert_eq!(canonical_digest(&packet.record().packet).unwrap().0,digest);
            assert_eq!(packet.record().packet,document(COMPILED[index].packet).unwrap());
            assert_eq!(packet.record().evidence_documents,document(COMPILED[index].documents).unwrap());
            assert_eq!(packet.record().ingredient_registry,document(COMPILED[index].registry).unwrap());
            assert_eq!(packet.record().source_captures,document(COMPILED[index].captures).unwrap());
        }
    }
}
