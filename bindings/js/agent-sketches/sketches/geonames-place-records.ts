// GeoNames' published gazetteer export: place rows, name variants, hierarchy
// links, user tags and the reference files that interpret codes on a place.
// Sources: https://download.geonames.org/export/dump/readme.txt
// https://www.geonames.org/export/index.html

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "table" | "group";
  x: number;
  y: number;
  label: string;
  parent?: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "features", class: "group", x: 20, y: 20, width: 680, height: 630, label: "Feature records and linked data", fill: "#eff6ff" },
    { id: "reference", class: "group", x: 720, y: 20, width: 500, height: 630, label: "Code and geometry reference files", fill: "#fefce8" },
    { id: "geoname", class: "table", x: 60, y: 90, parent: "features", label: "geoname\nPK geonameid\nname · asciiname · latitude · longitude\nfeature class · feature code · country code\nadmin1–4 codes · population · elevation\ntimezone · modification date" },
    { id: "alternate-names", class: "table", x: 60, y: 310, parent: "features", label: "alternateNamesV2\nPK alternateNameId · geonameId FK\nisoLanguage · alternateName\nisPreferred · isShort · isColloquial · isHistoric\nfrom · to" },
    { id: "hierarchy", class: "table", x: 390, y: 100, parent: "features", label: "hierarchy\nparentId · childId · type\nADM = admin hierarchy\nother links entered by users" },
    { id: "user-tags", class: "table", x: 390, y: 300, parent: "features", label: "userTags\ngeonameId · tag" },
    { id: "admin1", class: "table", x: 760, y: 100, parent: "reference", label: "admin1CodesASCII\ncode · name · asciiname\ngeonameId" },
    { id: "admin2", class: "table", x: 1000, y: 100, parent: "reference", label: "admin2Codes\ncode · name · asciiname\ngeonameId" },
    { id: "country", class: "table", x: 760, y: 260, parent: "reference", label: "countryInfo\nISO codes · country names\nlanguages · capital" },
    { id: "feature-codes", class: "table", x: 1000, y: 260, parent: "reference", label: "featureCodes\nfeature class/code\nname · description" },
    { id: "time-zones", class: "table", x: 760, y: 420, parent: "reference", label: "timeZones\ncountryCode · timezoneId\nGMT offset · DST offset" },
    { id: "boundaries", class: "table", x: 1000, y: 420, parent: "reference", label: "shapes_simplified_low\ngeonameId · geoJson\nsimplified country boundaries" },
  ] satisfies Node[],
  edges: [
    { from: "alternate-names", to: "geoname", label: "geonameId" },
    { from: "hierarchy", to: "geoname", label: "parentId / childId" },
    { from: "user-tags", to: "geoname", label: "geonameId" },
    { from: "admin1", to: "geoname", label: "countryCode + admin1 code; matching ADM1 feature" },
    { from: "admin2", to: "geoname", label: "admin2 code" },
    { from: "country", to: "geoname", label: "country code" },
    { from: "feature-codes", to: "geoname", label: "feature class + feature code" },
    { from: "time-zones", to: "geoname", label: "timezoneId" },
    { from: "boundaries", to: "geoname", label: "geonameId" },
  ],
  style: { table: box, group: box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
