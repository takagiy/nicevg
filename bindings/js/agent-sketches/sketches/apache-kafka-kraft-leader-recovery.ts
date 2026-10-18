// Apache Kafka KRaft leader recovery.
// Sources: https://github.com/apache/kafka/blob/trunk/raft/src/main/java/org/apache/kafka/raft/KafkaRaftClient.java
// https://github.com/apache/kafka/blob/trunk/raft/src/main/java/org/apache/kafka/raft/RaftClient.java
// https://github.com/apache/kafka/blob/trunk/docs/operations/kraft.md
// Shows how a failed metadata quorum leader is replaced, followers reconcile their logs,
// and the new leader applies committed metadata before serving controller work.
import type { Sketch } from "nicevg";
import { svg, text } from "nicevg";

type N = { id: string; class: "proc" | "fn" | "state" | "msg" | "store"; x: number; y: number; label: string; parent?: string };
const proc = ({ label }: N) => { const t = text(label, 13); const w = t.width + 28, h = t.height + 18; return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="7" fill="#e0f2fe" stroke="#0369a1"/>${t.at({ cx: w / 2, cy: h / 2 })}`; };
const fn = ({ label }: N) => { const t = text(label, 12); const w = t.width + 24, h = t.height + 16; return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="2" fill="#fff" stroke="#475569"/>${t.at({ cx: w / 2, cy: h / 2 })}`; };
const state = ({ label }: N) => { const t = text(label, 12); const w = t.width + 24, h = t.height + 18; return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="${h / 2}" fill="#fef3c7" stroke="#b45309"/>${t.at({ cx: w / 2, cy: h / 2 })}`; };
const msg = ({ label }: N) => { const t = text(label, 11); const w = t.width + 20, h = t.height + 14; return svg`<path data-bounds="true" d="M 10 0 H ${w - 10} L ${w} ${h / 2} L ${w - 10} ${h} H 10 L 0 ${h / 2} Z" fill="#f3e8ff" stroke="#7e22ce"/>${t.at({ cx: w / 2, cy: h / 2 })}`; };
const store = ({ label }: N) => { const t = text(label, 11); const w = t.width + 24, h = t.height + 18; return svg`<path data-bounds="true" d="M 0 7 Q ${w / 2} -4 ${w} 7 V ${h - 7} Q ${w / 2} ${h + 4} 0 ${h - 7} Z M 0 7 Q ${w / 2} 18 ${w} 7 M 0 ${h - 7} Q ${w / 2} ${h + 4} ${w} ${h - 7}" fill="#dcfce7" stroke="#15803d"/>${t.at({ cx: w / 2, cy: h / 2 })}`; };

const nodes: N[] = [
  {id:"raft",class:"proc",x:450,y:30,label:"KafkaRaftClient"},
  {id:"poll",class:"fn",x:40,y:30,label:"poll / pollCurrentState"},
  {id:"timer",class:"fn",x:40,y:110,label:"election timeout"},
  {id:"follower",class:"state",x:40,y:190,label:"FollowerState"},
  {id:"unattached",class:"state",x:210,y:190,label:"UnattachedState"},
  {id:"prospective",class:"state",x:390,y:190,label:"ProspectiveState"},
  {id:"candidate",class:"state",x:590,y:190,label:"CandidateState"},
  {id:"leader",class:"state",x:790,y:190,label:"LeaderState"},
  {id:"resigned",class:"state",x:970,y:190,label:"ResignedState"},
  {id:"voteReq",class:"msg",x:210,y:300,label:"VoteRequestData: last offset + epoch"},
  {id:"voteResp",class:"msg",x:410,y:300,label:"VoteResponseData: granted / denied"},
  {id:"beginReq",class:"msg",x:640,y:300,label:"BeginQuorumEpochRequestData"},
  {id:"beginResp",class:"msg",x:850,y:300,label:"BeginQuorumEpochResponseData"},
  {id:"endReq",class:"msg",x:1030,y:300,label:"EndQuorumEpochRequestData"},
  {id:"fetchReq",class:"msg",x:210,y:390,label:"FetchRequestData"},
  {id:"fetchResp",class:"msg",x:390,y:390,label:"FetchResponseData: batches + HW"},
  {id:"snapshotReq",class:"msg",x:650,y:390,label:"FetchSnapshotRequestData"},
  {id:"snapshotResp",class:"msg",x:860,y:390,label:"FetchSnapshotResponseData: chunks"},
  {id:"requestMgr",class:"fn",x:40,y:480,label:"RequestManager / retry backoff"},
  {id:"sender",class:"fn",x:270,y:480,label:"DefaultRequestSender"},
  {id:"network",class:"proc",x:510,y:480,label:"Raft request channel"},
  {id:"voterA",class:"proc",x:760,y:480,label:"voter A (failed leader)"},
  {id:"voterB",class:"proc",x:960,y:480,label:"voter B (candidate)"},
  {id:"voterC",class:"proc",x:1160,y:480,label:"voter C (follower)"},
  {id:"quorum",class:"fn",x:270,y:570,label:"QuorumState"},
  {id:"voteState",class:"fn",x:40,y:570,label:"ElectionState"},
  {id:"voterSet",class:"store",x:40,y:660,label:"VoterSet"},
  {id:"logState",class:"fn",x:510,y:570,label:"KRaftControlRecordStateMachine"},
  {id:"metadataLog",class:"store",x:510,y:660,label:"Replicated metadata log"},
  {id:"snapshot",class:"store",x:760,y:660,label:"Raft snapshot (offset, epoch)"},
  {id:"highWatermark",class:"fn",x:960,y:570,label:"updateLeaderHighWatermark"},
  {id:"leaderChange",class:"fn",x:1160,y:570,label:"maybeFireLeaderChange"},
  {id:"listener",class:"fn",x:1160,y:660,label:"RaftClient.Listener"},
  {id:"loadSnapshot",class:"fn",x:960,y:750,label:"handleLoadSnapshot"},
  {id:"loadBootstrap",class:"fn",x:760,y:750,label:"handleLoadBootstrap"},
  {id:"handleCommit",class:"fn",x:510,y:750,label:"handleCommit(BatchReader)"},
  {id:"controller",class:"proc",x:270,y:750,label:"QuorumController state machine"},
  {id:"append",class:"fn",x:40,y:750,label:"prepareAppend(epoch, records)"},
  {id:"schedule",class:"fn",x:40,y:840,label:"schedulePreparedAppend"},
  {id:"batch",class:"fn",x:270,y:840,label:"BatchAccumulator"},
  {id:"purgatory",class:"fn",x:510,y:840,label:"FuturePurgatory / fetch waiters"},
  {id:"truncate",class:"fn",x:760,y:840,label:"truncate divergent tail on epoch change"},
  {id:"replay",class:"fn",x:960,y:840,label:"replay committed records"},
  {id:"ready",class:"state",x:1160,y:840,label:"leader ready after HW apply"},
  {id:"loss",class:"state",x:40,y:930,label:"leader process / link fails"},
  {id:"noQuorum",class:"state",x:270,y:930,label:"no majority: writes cannot commit"},
  {id:"newEpoch",class:"fn",x:510,y:930,label:"advance epoch; reject stale leader"},
  {id:"notLeader",class:"msg",x:760,y:930,label:"NotLeaderException"},
  {id:"discard",class:"fn",x:960,y:930,label:"discard uncommitted old-epoch entries"},
  {id:"recover",class:"state",x:1160,y:930,label:"failed voter restarts; log + snapshot reload"},
  {id:"metrics",class:"fn",x:40,y:1020,label:"KafkaRaftMetrics: election latency"},
  {id:"describe",class:"fn",x:270,y:1020,label:"DescribeQuorum request"},
  {id:"leaderInfo",class:"store",x:510,y:1020,label:"leader id / epoch / high watermark"},
  {id:"appendRecords",class:"fn",x:760,y:1020,label:"metadata record append"},
  {id:"controlRecords",class:"store",x:960,y:1020,label:"VotersRecord / KRaftVersionRecord"},
  {id:"observer",class:"state",x:1160,y:1020,label:"observer: replicates, never votes"},
];

const edgePairs: [string, string][] = [
 ["poll","raft"],["timer","poll"],["follower","timer"],["unattached","timer"],["prospective","quorum"],["candidate","quorum"],["leader","quorum"],["resigned","quorum"],
 ["poll","voteReq"],["voteReq","voterA"],["voteReq","voterC"],["voteReq","voterB"],["voterA","voteResp"],["voterC","voteResp"],["voteResp","quorum"],["quorum","voteState"],["voteState","voterSet"],
 ["prospective","candidate"],["candidate","leader"],["leader","beginReq"],["beginReq","voterA"],["beginReq","voterC"],["beginResp","leader"],["voterA","beginResp"],["voterC","beginResp"],
 ["leader","fetchResp"],["fetchReq","voterB"],["fetchReq","voterC"],["voterB","fetchResp"],["voterC","fetchResp"],["fetchResp","logState"],["fetchReq","snapshotReq"],["snapshotReq","voterA"],["snapshotResp","snapshot"],["snapshot","logState"],
 ["raft","requestMgr"],["requestMgr","sender"],["sender","network"],["network","voterA"],["network","voterB"],["network","voterC"],["network","requestMgr"],
 ["logState","metadataLog"],["logState","highWatermark"],["highWatermark","listener"],["leaderChange","listener"],["listener","loadSnapshot"],["listener","loadBootstrap"],["listener","handleCommit"],["loadSnapshot","controller"],["loadBootstrap","controller"],["handleCommit","controller"],
 ["controller","append"],["append","batch"],["batch","schedule"],["schedule","metadataLog"],["metadataLog","purgatory"],["purgatory","fetchResp"],["leader","loss"],["loss","timer"],["timer","noQuorum"],["noQuorum","voteReq"],
 ["leader","newEpoch"],["newEpoch","notLeader"],["notLeader","append"],["newEpoch","discard"],["discard","truncate"],["truncate","metadataLog"],["highWatermark","replay"],["replay","ready"],["ready","controller"],
 ["voterA","recover"],["recover","snapshot"],["recover","metadataLog"],["candidate","metrics"],["metrics","leaderInfo"],["leaderInfo","describe"],["controller","appendRecords"],["appendRecords","controlRecords"],["controlRecords","logState"],["observer","fetchReq"],["endReq","voterB"],["resigned","endReq"],
];
const edges = edgePairs.map(([from, to]) => ({ from, to }));

export default { nodes, edges, style: { proc, fn, state, msg, store }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<N>;
