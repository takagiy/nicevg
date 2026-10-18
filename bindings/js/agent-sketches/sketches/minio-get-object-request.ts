// MinIO. Sources:
// https://github.com/minio/minio/blob/master/cmd/api-router.go
// https://github.com/minio/minio/blob/master/cmd/object-handlers.go
// https://github.com/minio/minio/blob/master/cmd/erasure-server-pool.go
// Shows an S3 GET /{bucket}/{object} from route selection through authorization,
// range and conditional checks, local or replication-target reads, and the HTTP response.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "client", class: "box", x: 20, y: 150, label: "S3 client\nGET /bucket/key\nRange, If-Match, auth headers", width: 210, height: 72 },
    { id: "route", class: "box", x: 280, y: 150, label: "api-router.go\nGET /{object:.+}\nGetObjectHandler", width: 220, height: 70 },
    { id: "middleware", class: "box", x: 560, y: 150, label: "s3APIMiddleware\nS3 request context, tracing\nrequest authentication", width: 230, height: 72 },
    { id: "handler", class: "box", x: 850, y: 150, label: "GetObjectHandler\nunescape bucket + object\nselect archive or regular GET", width: 240, height: 72 },
    { id: "opts", class: "box", x: 1150, y: 150, label: "getObjectHandler\ngetOpts: version, part, conditions\nauthenticate GetObject", width: 245, height: 76 },
    { id: "range", class: "box", x: 1150, y: 350, label: "Range handling\nparse Range → HTTPRangeSpec\nreject invalid range / part pair", width: 250, height: 76 },
    { id: "pool", class: "box", x: 850, y: 350, label: "erasureServerPools\nGetObjectNInfo\nread lock + latest object info", width: 245, height: 76 },
    { id: "select", class: "box", x: 560, y: 350, label: "server pool selection\nobject info + pool index\nversion / delete marker", width: 230, height: 72 },
    { id: "reader", class: "box", x: 280, y: 350, label: "GetObjectReader\nselected pool GetObjectNInfo\nobject bytes + ObjInfo", width: 220, height: 72 },
    { id: "proxy", class: "box", x: 850, y: 560, label: "Replication proxy fallback\nmissing / version missing / read quorum\nproxyGetToReplicationTarget", width: 270, height: 76 },
    { id: "checks", class: "box", x: 560, y: 560, label: "CheckPrecondFn\nDecryptObjectInfo → authorizeRequest\ncheckPreconditions (ETag / dates)", width: 260, height: 76 },
    { id: "stream", class: "box", x: 280, y: 560, label: "setObjectHeaders + httpWriter\nmetadata, range, status\nxioutil.Copy(reader → response)", width: 240, height: 76 },
    { id: "response", class: "box", x: 20, y: 560, label: "S3 client receives\nHTTP headers + object body\nor S3 XML error response", width: 210, height: 72 },
  ],
  edges: [
    { from: "client", to: "route", label: "HTTP request" },
    { from: "route", to: "middleware", label: "matched route handler" },
    { from: "middleware", to: "handler", label: "authenticated request + mux vars" },
    { from: "handler", to: "opts", label: "bucket, decoded object, request" },
    { from: "opts", to: "range", label: "ObjectOptions; GetObject authorized" },
    { from: "range", to: "pool", label: "HTTPRangeSpec + request headers" },
    { from: "pool", to: "select", label: "GetObjectNInfo; inspect pools" },
    { from: "select", to: "reader", label: "selected pool + object/version" },
    { from: "reader", to: "checks", label: "reader + ObjInfo" },
    { from: "checks", to: "stream", label: "preconditions pass; object metadata" },
    { from: "stream", to: "response", label: "streamed bytes + S3 headers" },
    { from: "reader", to: "proxy", label: "not found / version missing / read quorum" },
    { from: "proxy", to: "checks", label: "replication-target reader, if found" },
    { from: "opts", to: "response", label: "auth / options error → writeErrorResponse" },
    { from: "range", to: "response", label: "invalid range → S3 error" },
    { from: "checks", to: "response", label: "denied / failed condition → S3 error" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
