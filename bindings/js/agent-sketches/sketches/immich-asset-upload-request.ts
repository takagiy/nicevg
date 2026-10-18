// Immich asset upload request path.
// Sources: https://docs.immich.app/developer/architecture/,
// https://github.com/immich-app/immich/blob/main/server/src/controllers/asset-media.controller.ts,
// https://github.com/immich-app/immich/blob/main/server/src/middleware/asset-upload.interceptor.ts,
// https://github.com/immich-app/immich/blob/main/server/src/services/asset-media.service.ts,
// https://github.com/immich-app/immich/blob/main/server/src/services/asset.service.ts,
// https://github.com/immich-app/immich/blob/main/server/src/repositories/asset.repository.ts,
// https://github.com/immich-app/immich/blob/main/server/src/repositories/job.repository.ts
// Shows a mobile client's multipart upload through NestJS, validation, asset persistence and job
// queuing to the upload response; the worker path is included because it is triggered by the request.

import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
  x: number;
  y: number;
  parent?: string;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const nodes: Node[] = [
  { id: "mobile", class: "circle", x: 20, y: 300, label: "モバイルアプリ\nmultipart POST" },
  { id: "http", class: "box", x: 210, y: 300, label: "HTTP クライアント\n/api/assets" },
  { id: "server", class: "box", x: 410, y: 300, label: "immich-server\nNestJS / Express" },
  { id: "route", class: "box", x: 620, y: 300, label: "AssetMediaController\nPOST /upload" },
  { id: "auth", class: "box", x: 830, y: 120, label: "AuthGuard\nAuthDto を設定" },
  { id: "unauth", class: "box", x: 1030, y: 20, label: "認証エラー\n401 応答" },
  { id: "interceptor", class: "box", x: 830, y: 300, label: "AssetUploadInterceptor\nmultipart を処理" },
  { id: "filefilter", class: "box", x: 1030, y: 180, label: "ファイル種別・サイズ\n検査" },
  { id: "badfile", class: "box", x: 1230, y: 20, label: "BadRequestException\n4xx 応答" },
  { id: "dto", class: "box", x: 1030, y: 370, label: "AssetMediaCreateDto\nchecksum・日時・端末" },
  { id: "controller", class: "box", x: 1230, y: 300, label: "uploadAsset(auth, dto,\nfile, sidecarFile)" },
  { id: "mediaSvc", class: "box", x: 1440, y: 300, label: "AssetMediaService\nuploadAsset" },
  { id: "validate", class: "box", x: 1640, y: 140, label: "重複・整合性確認\nchecksum / owner" },
  { id: "duplicate", class: "box", x: 1840, y: 20, label: "既存アセット\nAssetResponseDto" },
  { id: "storage", class: "box", x: 1840, y: 300, label: "StorageCore\n保存先パスを決定" },
  { id: "filewrite", class: "box", x: 2050, y: 300, label: "ファイルシステム\n原本・sidecar" },
  { id: "assetSvc", class: "box", x: 2260, y: 300, label: "AssetService\ncreateAsset" },
  { id: "accessRepo", class: "box", x: 2260, y: 140, label: "AccessRepository\nユーザー権限" },
  { id: "assetRepo", class: "box", x: 2470, y: 300, label: "AssetRepository\nasset を作成" },
  { id: "postgres", class: "box", x: 2680, y: 300, label: "Postgres\nasset・asset_file" },
  { id: "jobs", class: "box", x: 2470, y: 500, label: "AssetJobService\n初期ジョブを登録" },
  { id: "jobrepo", class: "box", x: 2680, y: 500, label: "JobRepository\nBullMQ に enqueue" },
  { id: "redis", class: "box", x: 2890, y: 500, label: "Redis\nジョブキュー" },
  { id: "worker", class: "box", x: 3100, y: 500, label: "immich-microservices\nジョブ worker" },
  { id: "metadata", class: "box", x: 3310, y: 400, label: "メタデータ抽出\nサムネイル生成" },
  { id: "ml", class: "box", x: 3310, y: 600, label: "後続ジョブ\nSmart Search / 顔認識" },
  { id: "result", class: "box", x: 2890, y: 180, label: "AssetResponseDto\nasset id・status" },
  { id: "reply", class: "box", x: 3100, y: 180, label: "Controller 応答\nHTTP 201" },
  { id: "client", class: "circle", x: 3310, y: 180, label: "アプリが\nasset id を受信" },
];

const edges = [
  { from: "mobile", to: "http", label: "POST + multipart" },
  { from: "http", to: "server", label: "HTTP" },
  { from: "server", to: "route", label: "URL routing" },
  { from: "route", to: "auth", label: "guard" },
  { from: "auth", to: "unauth", label: "token 不正" },
  { from: "auth", to: "interceptor", label: "AuthDto" },
  { from: "interceptor", to: "filefilter", label: "uploaded file" },
  { from: "filefilter", to: "badfile", label: "拒否" },
  { from: "filefilter", to: "dto", label: "file・sidecar・fields" },
  { from: "dto", to: "controller", label: "検証済み入力" },
  { from: "route", to: "controller", label: "handler 呼び出し" },
  { from: "controller", to: "mediaSvc", label: "auth + DTO + files" },
  { from: "mediaSvc", to: "validate", label: "asset 重複確認" },
  { from: "validate", to: "duplicate", label: "重複あり" },
  { from: "duplicate", to: "result", label: "既存レコード" },
  { from: "validate", to: "storage", label: "新規アップロード" },
  { from: "storage", to: "filewrite", label: "移動・保存" },
  { from: "filewrite", to: "assetSvc", label: "保存済みパス + 情報" },
  { from: "assetSvc", to: "accessRepo", label: "所有者・アクセス確認" },
  { from: "assetSvc", to: "assetRepo", label: "asset 作成・更新" },
  { from: "assetRepo", to: "postgres", label: "asset / asset_file" },
  { from: "postgres", to: "assetRepo", label: "保存済み asset" },
  { from: "assetRepo", to: "assetSvc", label: "AssetEntity" },
  { from: "assetSvc", to: "jobs", label: "asset 作成イベント" },
  { from: "jobs", to: "jobrepo", label: "metadata・thumbnail jobs" },
  { from: "jobrepo", to: "redis", label: "BullMQ job data" },
  { from: "redis", to: "worker", label: "queued jobs" },
  { from: "worker", to: "metadata", label: "asset を処理" },
  { from: "metadata", to: "ml", label: "thumbnail 完了後" },
  { from: "assetSvc", to: "result", label: "AssetResponseDto" },
  { from: "result", to: "reply", label: "DTO" },
  { from: "reply", to: "client", label: "HTTP 201 + JSON" },
  { from: "unauth", to: "client", label: "401" },
  { from: "badfile", to: "client", label: "4xx" },
];

export default {
  nodes,
  edges,
  style: { box, circle },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
