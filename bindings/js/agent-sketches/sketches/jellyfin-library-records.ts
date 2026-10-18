/**
 * Jellyfin library records:
 * https://github.com/jellyfin/jellyfin/blob/master/MediaBrowser.Model/Dto/BaseItemDto.cs
 * https://github.com/jellyfin/jellyfin/blob/master/src/Jellyfin.Database/Jellyfin.Database.Implementations/Entities/UserData.cs
 * https://github.com/jellyfin/jellyfin/blob/master/src/Jellyfin.Database/Jellyfin.Database.Implementations/Entities/MediaStreamInfo.cs
 * This slice explains how a library item is placed in the parent tree, how
 * ancestor lookup is materialized, and where streams, provider ids and user state attach.
 */
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "base-items", class: "box", x: 50, y: 160, label: "BaseItems\nId (PK) · Name · Type · Path\nParentId · TopParentId · SeriesId" },
    { id: "ancestors", class: "box", x: 350, y: 55, label: "AncestorIds\nItemId · ParentItemId\nmaterialized item / ancestor pairs" },
    { id: "streams", class: "box", x: 350, y: 180, label: "MediaStreamInfos\nItemId · StreamIndex\nType · Codec · Language · BitRate" },
    { id: "providers", class: "box", x: 350, y: 310, label: "BaseItemProviders\nItemId · ProviderId · Provider\nexternal metadata identity" },
    { id: "user-data", class: "box", x: 690, y: 180, label: "UserData\nItemId · UserId (key pair)\nPlayed · PlaybackPositionTicks\nFavorite · PlayCount" },
  ] satisfies Node[],
  edges: [
    { from: "base-items", to: "base-items", label: "ParentId · child → parent" },
    { from: "base-items", to: "ancestors", label: "Id → ItemId; Id → ParentItemId" },
    { from: "base-items", to: "streams", label: "Id → ItemId · tracks for item" },
    { from: "base-items", to: "providers", label: "Id → ItemId · external ids" },
    { from: "base-items", to: "user-data", label: "Id → ItemId · per-user state" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
