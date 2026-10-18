// PostgreSQL 18 system catalogs: https://www.postgresql.org/docs/18/catalogs.html
// Catalog pages: pg_class, pg_attribute, pg_type, pg_constraint, pg_index,
// pg_depend, pg_namespace, pg_authid, pg_shdepend, pg_rewrite, pg_trigger,
// pg_proc, pg_language, pg_operator, pg_am, pg_opclass, pg_collation,
// pg_sequence, pg_attrdef, pg_partitioned_table, pg_inherits.
// This sketch shows how PostgreSQL stores database object definitions and
// links relation, column, type, constraint, index, ownership and dependency records.
import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

const nodes = [
  { id: "namespace-area", class: "box", x: 20, y: 20, width: 840, height: 160, label: "Names and ownership", fill: "#e0f2fe" },
  { id: "relation-area", class: "box", x: 20, y: 205, width: 840, height: 340, label: "Relations, columns and storage definitions", fill: "#ecfccb" },
  { id: "definition-area", class: "box", x: 20, y: 570, width: 840, height: 300, label: "Types, constraints and executable definitions", fill: "#fef3c7" },
  { id: "dependency-area", class: "box", x: 890, y: 20, width: 670, height: 850, label: "Links, dependencies and shared objects", fill: "#fce7f3" },

  { id: "pg_namespace", class: "box", parent: "namespace-area", x: 50, y: 70, label: "pg_namespace\noid PK · nspname\nnspowner · nspacl" },
  { id: "pg_authid", class: "box", parent: "namespace-area", x: 340, y: 70, label: "pg_authid\n oid PK · rolname\nrole flags · rolconfig" },
  { id: "pg_database", class: "box", parent: "namespace-area", x: 600, y: 70, label: "pg_database\noid PK · datname\ndatdba · encoding" },

  { id: "pg_class", class: "box", parent: "relation-area", x: 50, y: 265, label: "pg_class\noid PK · relname\nrelnamespace · reltype\nrelkind · relowner · relfilenode" },
  { id: "pg_attribute", class: "box", parent: "relation-area", x: 340, y: 265, label: "pg_attribute\n(attrelid, attnum) PK\nattname · atttypid\natnotnull · attisdropped" },
  { id: "pg_index", class: "box", parent: "relation-area", x: 625, y: 265, label: "pg_index\nindexrelid PK · indrelid\nindkey · indisprimary\nindisunique · indvalid" },
  { id: "pg_sequence", class: "box", parent: "relation-area", x: 50, y: 400, label: "pg_sequence\nseqrelid PK · seqtypid\nseqstart · seqincrement\nseqmin · seqmax · seqcache" },
  { id: "pg_partitioned_table", class: "box", parent: "relation-area", x: 340, y: 400, label: "pg_partitioned_table\npartrelid PK · partstrat\npartattrs · partclass\npartexprs" },
  { id: "pg_inherits", class: "box", parent: "relation-area", x: 625, y: 400, label: "pg_inherits\n(inhrelid, inhparent)\ninhseqno · child-to-parent\npartition / inheritance link" },

  { id: "pg_type", class: "box", parent: "definition-area", x: 50, y: 630, label: "pg_type\noid PK · typname\ntypnamespace · typrelid\ntypbasetype · typtype" },
  { id: "pg_constraint", class: "box", parent: "definition-area", x: 340, y: 630, label: "pg_constraint\noid PK · conname · contype\nconrelid · confrelid\nconkey · confkey · conindid" },
  { id: "pg_attrdef", class: "box", parent: "definition-area", x: 625, y: 630, label: "pg_attrdef\noid PK · adrelid · adnum\nadbin expression tree" },
  { id: "pg_proc", class: "box", parent: "definition-area", x: 50, y: 755, label: "pg_proc\noid PK · proname\nprorettype · proargtypes\nprolang · prosrc" },
  { id: "pg_language", class: "box", parent: "definition-area", x: 340, y: 755, label: "pg_language\noid PK · lanname\nlanplcallfoid · laninline" },
  { id: "pg_operator", class: "box", parent: "definition-area", x: 625, y: 755, label: "pg_operator\noid PK · oprname\noprcode · oprleft · oprright" },

  { id: "pg_depend", class: "box", parent: "dependency-area", x: 930, y: 80, label: "pg_depend\n(classid, objid, objsubid,\nrefclassid, refobjid, refobjsubid)\ndeptype: normal / auto / internal" },
  { id: "pg_shdepend", class: "box", parent: "dependency-area", x: 1240, y: 80, label: "pg_shdepend\ndbid · classid · objid\nrefclassid · refobjid · deptype\nrole / tablespace references" },
  { id: "pg_rewrite", class: "box", parent: "dependency-area", x: 930, y: 240, label: "pg_rewrite\noid PK · ev_class\nrulename · ev_type\nev_qual · ev_action trees" },
  { id: "pg_trigger", class: "box", parent: "dependency-area", x: 1240, y: 240, label: "pg_trigger\noid PK · tgrelid\ntgfoid · tgtype · tgargs" },
  { id: "pg_am", class: "box", parent: "dependency-area", x: 930, y: 400, label: "pg_am\noid PK · amname\namtype · handler" },
  { id: "pg_opclass", class: "box", parent: "dependency-area", x: 1240, y: 400, label: "pg_opclass\noid PK · opcmethod\nopcnamespace · opcintype\noperator class for index keys" },
  { id: "pg_collation", class: "box", parent: "dependency-area", x: 930, y: 560, label: "pg_collation\noid PK · collname\ncollnamespace · collprovider\nlocale / collation version" },
  { id: "pg_tablespace", class: "box", parent: "dependency-area", x: 1240, y: 560, label: "pg_tablespace\noid PK · spcname\nspcowner · spcacl" },
  { id: "pg_policy", class: "box", parent: "dependency-area", x: 930, y: 720, label: "pg_policy\n(polrelid, polname)\npolcmd · polroles · polqual\npolwithcheck" },
  { id: "pg_statistic_ext", class: "box", parent: "dependency-area", x: 1240, y: 720, label: "pg_statistic_ext\noid PK · stxrelid · stxname\nstxkeys · stxkind\nextended column statistics" },
] satisfies SketchNode[];

type SketchNode = { id: string; class: "box"; x: number; y: number; width?: number; height?: number; label: string; fill?: string; parent?: string };

const edges = [
  { from: "pg_class", to: "pg_namespace", label: "relnamespace" },
  { from: "pg_namespace", to: "pg_authid", label: "nspowner" },
  { from: "pg_class", to: "pg_type", label: "reltype row type" },
  { from: "pg_attribute", to: "pg_class", label: "attrelid" },
  { from: "pg_attribute", to: "pg_type", label: "atttypid" },
  { from: "pg_index", to: "pg_class", label: "indexrelid + indrelid" },
  { from: "pg_index", to: "pg_attribute", label: "indkey attnum array" },
  { from: "pg_sequence", to: "pg_class", label: "seqrelid" },
  { from: "pg_partitioned_table", to: "pg_class", label: "partrelid" },
  { from: "pg_inherits", to: "pg_class", label: "inhrelid / inhparent" },
  { from: "pg_type", to: "pg_namespace", label: "typnamespace" },
  { from: "pg_type", to: "pg_class", label: "typrelid composite type" },
  { from: "pg_constraint", to: "pg_class", label: "conrelid / confrelid" },
  { from: "pg_constraint", to: "pg_index", label: "conindid supporting index" },
  { from: "pg_constraint", to: "pg_type", label: "contypid domain constraint" },
  { from: "pg_attrdef", to: "pg_attribute", label: "adrelid + adnum" },
  { from: "pg_proc", to: "pg_type", label: "prorettype / proargtypes" },
  { from: "pg_proc", to: "pg_language", label: "prolang" },
  { from: "pg_operator", to: "pg_proc", label: "oprcode implementation" },
  { from: "pg_rewrite", to: "pg_class", label: "ev_class view/table" },
  { from: "pg_trigger", to: "pg_class", label: "tgrelid" },
  { from: "pg_trigger", to: "pg_proc", label: "tgfoid function" },
  { from: "pg_index", to: "pg_am", label: "access method via pg_class.relam" },
  { from: "pg_index", to: "pg_opclass", label: "indclass operator classes" },
  { from: "pg_index", to: "pg_collation", label: "indcollation" },
  { from: "pg_class", to: "pg_tablespace", label: "reltablespace" },
  { from: "pg_depend", to: "pg_class", label: "classid / objid / objsubid" },
  { from: "pg_depend", to: "pg_type", label: "refclassid / refobjid / subid" },
  { from: "pg_shdepend", to: "pg_authid", label: "role in refobjid" },
  { from: "pg_shdepend", to: "pg_class", label: "dependent object's catalog address" },
  { from: "pg_shdepend", to: "pg_tablespace", label: "shared tablespace reference" },
  { from: "pg_database", to: "pg_authid", label: "datdba owner" },
  { from: "pg_policy", to: "pg_class", label: "polrelid" },
  { from: "pg_statistic_ext", to: "pg_class", label: "stxrelid" },
];

export default { nodes, edges, style: { box } } satisfies Sketch<SketchNode>;
