// OpenMRS data model: https://docs.openmrs.org/datamodel/openmrs-data-model-3.4.0.html
// Information model: https://guide.openmrs.org/getting-started/openmrs-information-model/
// This slice follows a patient's clinical record from person and patient identity through visits,
// encounters, observations and orders, with the metadata and location records those records reference.
import { svg, text } from "nicevg";
import type { Sketch } from "nicevg";

type Column = [name: string, type: string, key?: "PK" | "FK"];
type Table = { id: string; class: "table"; x: number; y: number; parent: string; columns: Column[] };
type Area = { id: string; class: "area"; x: number; y: number; title: string; width: number; height: number; fill: string };
const FONT = 11, HEAD = 24, ROW = 18, PAD = 7, KEY = 24;
const table = ({ id, columns }: Table) => {
  const title = text(id, 12), names = columns.map(([n]) => text(n, FONT)), types = columns.map(([, t]) => text(t, FONT));
  const width = Math.max(title.width + PAD * 2, KEY + Math.max(...names.map(n => n.width)) + Math.max(...types.map(t => t.width)) + PAD * 3);
  const height = HEAD + columns.length * ROW, cy = (i: number) => HEAD + i * ROW + ROW / 2;
  return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="white" stroke="#475569"/><rect width="${width}" height="${HEAD}" fill="#dbeafe" stroke="#475569"/>${title.at({ cx: width / 2, cy: HEAD / 2 })}${columns.map(([n,,k],i) => [k ? text(k,9).at({left:PAD,cy:cy(i)}) : "", names[i]!.at({left:KEY,cy:cy(i)}), types[i]!.at({right:width-PAD,cy:cy(i)}), [0,width].map(x => svg`<circle data-port="${n}" cx="${x}" cy="${cy(i)}" r="0"/>`)])}`;
};
const area = ({ title, width, height, fill }: Area) => svg`<rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#94a3b8"/>${text(title,13).at({left:10,top:7})}`;
const tables: Omit<Table,"class">[] = [
 {id:"person",parent:"identity",x:30,y:45,columns:[["person_id","int", "PK"],["gender","char"],["birthdate","date"],["dead","boolean"],["voided","boolean"]]},
 {id:"patient",parent:"identity",x:30,y:170,columns:[["patient_id","int","PK"],["person_id","int","FK"],["voided","boolean"]]},
 {id:"person_name",parent:"identity",x:260,y:45,columns:[["person_name_id","int","PK"],["person_id","int","FK"],["given_name","varchar"],["family_name","varchar"],["preferred","boolean"]]},
 {id:"person_address",parent:"identity",x:260,y:170,columns:[["person_address_id","int","PK"],["person_id","int","FK"],["address1","varchar"],["city_village","varchar"],["preferred","boolean"]]},
 {id:"patient_identifier",parent:"identity",x:30,y:300,columns:[["patient_identifier_id","int","PK"],["patient_id","int","FK"],["identifier","varchar"],["identifier_type","int","FK"],["location_id","int","FK"]]},
 {id:"patient_identifier_type",parent:"identity",x:260,y:330,columns:[["patient_identifier_type_id","int","PK"],["name","varchar"],["format","varchar"],["required","boolean"]]},
 {id:"person_attribute",parent:"identity",x:30,y:470,columns:[["person_attribute_id","int","PK"],["person_id","int","FK"],["person_attribute_type_id","int","FK"],["value","varchar"]]},
 {id:"person_attribute_type",parent:"identity",x:260,y:490,columns:[["person_attribute_type_id","int","PK"],["name","varchar"],["format","varchar"],["foreign_key","varchar"]]},
 {id:"relationship",parent:"identity",x:30,y:620,columns:[["relationship_id","int","PK"],["person_a","int","FK"],["person_b","int","FK"],["relationship","int","FK"]]},
 {id:"relationship_type",parent:"identity",x:260,y:650,columns:[["relationship_type_id","int","PK"],["a_is_to_b","varchar"],["b_is_to_a","varchar"]]},
 {id:"visit",parent:"care",x:560,y:45,columns:[["visit_id","int","PK"],["patient_id","int","FK"],["visit_type_id","int","FK"],["location_id","int","FK"],["date_started","timestamp"],["date_stopped","timestamp"]]},
 {id:"visit_type",parent:"care",x:790,y:45,columns:[["visit_type_id","int","PK"],["name","varchar"],["description","varchar"]]},
 {id:"visit_attribute",parent:"care",x:560,y:205,columns:[["visit_attribute_id","int","PK"],["visit_id","int","FK"],["attribute_type_id","int","FK"],["value_reference","text"]]},
 {id:"visit_attribute_type",parent:"care",x:790,y:215,columns:[["visit_attribute_type_id","int","PK"],["name","varchar"],["datatype","varchar"]]},
 {id:"encounter",parent:"care",x:560,y:370,columns:[["encounter_id","int","PK"],["patient_id","int","FK"],["encounter_type","int","FK"],["visit_id","int","FK"],["location_id","int","FK"],["encounter_datetime","timestamp"],["form_id","int","FK"]]},
 {id:"encounter_type",parent:"care",x:790,y:385,columns:[["encounter_type_id","int","PK"],["name","varchar"],["description","varchar"]]},
 {id:"encounter_provider",parent:"care",x:560,y:555,columns:[["encounter_provider_id","int","PK"],["encounter_id","int","FK"],["provider_id","int","FK"],["encounter_role_id","int","FK"]]},
 {id:"provider",parent:"care",x:790,y:570,columns:[["provider_id","int","PK"],["person_id","int","FK"],["identifier","varchar"]]},
 {id:"obs",parent:"care",x:1060,y:45,columns:[["obs_id","int","PK"],["person_id","int","FK"],["concept_id","int","FK"],["encounter_id","int","FK"],["obs_group_id","int","FK"],["value_coded","int","FK"],["value_text","text"],["value_numeric","double"]]},
 {id:"orders",parent:"care",x:1060,y:250,columns:[["order_id","int","PK"],["patient_id","int","FK"],["concept_id","int","FK"],["order_type_id","int","FK"],["encounter_id","int","FK"],["care_setting","int","FK"],["previous_order_id","int","FK"]]},
 {id:"order_type",parent:"care",x:1300,y:250,columns:[["order_type_id","int","PK"],["name","varchar"],["java_class_name","varchar"]]},
 {id:"concept",parent:"metadata",x:1060,y:470,columns:[["concept_id","int","PK"],["datatype_id","int","FK"],["class_id","int","FK"],["retired","boolean"]]},
 {id:"concept_name",parent:"metadata",x:1300,y:430,columns:[["concept_name_id","int","PK"],["concept_id","int","FK"],["name","varchar"],["locale","varchar"],["concept_name_type","varchar"]]},
 {id:"concept_datatype",parent:"metadata",x:1060,y:635,columns:[["concept_datatype_id","int","PK"],["name","varchar"]]},
 {id:"concept_class",parent:"metadata",x:1300,y:600,columns:[["concept_class_id","int","PK"],["name","varchar"]]},
 {id:"location",parent:"metadata",x:560,y:740,columns:[["location_id","int","PK"],["name","varchar"],["parent_location","int","FK"],["retired","boolean"]]},
 {id:"form",parent:"metadata",x:790,y:740,columns:[["form_id","int","PK"],["name","varchar"],["version","varchar"],["retired","boolean"]]},
];
const areas: Area[] = [
 {id:"identity",class:"area",x:10,y:20,width:490,height:740,title:"Person and patient identity",fill:"#f0f9ff"},
 {id:"care",class:"area",x:530,y:20,width:500,height:700,title:"Visits, encounters and clinical records",fill:"#f0fdf4"},
 {id:"metadata",class:"area",x:1040,y:400,width:490,height:400,title:"Clinical and location metadata",fill:"#fffbeb"},
];
const nodes: (Table|Area)[] = [...areas,...tables.map(t=>({...t,class:"table" as const}))];
const edges = [
 ["patient.person_id","person.person_id"],["person_name.person_id","person.person_id"],["person_address.person_id","person.person_id"],["patient_identifier.patient_id","patient.patient_id"],["patient_identifier.identifier_type","patient_identifier_type.patient_identifier_type_id"],["patient_identifier.location_id","location.location_id"],["person_attribute.person_id","person.person_id"],["person_attribute.person_attribute_type_id","person_attribute_type.person_attribute_type_id"],["relationship.person_a","person.person_id"],["relationship.person_b","person.person_id"],["relationship.relationship","relationship_type.relationship_type_id"],
 ["visit.patient_id","patient.patient_id"],["visit.visit_type_id","visit_type.visit_type_id"],["visit.location_id","location.location_id"],["visit_attribute.visit_id","visit.visit_id"],["visit_attribute.attribute_type_id","visit_attribute_type.visit_attribute_type_id"],["encounter.patient_id","patient.patient_id"],["encounter.visit_id","visit.visit_id"],["encounter.encounter_type","encounter_type.encounter_type_id"],["encounter.location_id","location.location_id"],["encounter.form_id","form.form_id"],["encounter_provider.encounter_id","encounter.encounter_id"],["encounter_provider.provider_id","provider.provider_id"],["provider.person_id","person.person_id"],
 ["obs.person_id","person.person_id"],["obs.encounter_id","encounter.encounter_id"],["obs.concept_id","concept.concept_id"],["obs.obs_group_id","obs.obs_id"],["obs.value_coded","concept.concept_id"],["orders.patient_id","patient.patient_id"],["orders.encounter_id","encounter.encounter_id"],["orders.concept_id","concept.concept_id"],["orders.order_type_id","order_type.order_type_id"],["orders.previous_order_id","orders.order_id"],["concept.datatype_id","concept_datatype.concept_datatype_id"],["concept.class_id","concept_class.concept_class_id"],["concept_name.concept_id","concept.concept_id"],["location.parent_location","location.location_id"]
].map(pair=>({from:pair[0]!,to:pair[1]!}));
const sketch = { nodes, edges, style: { table, area }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Table|Area>;
export default sketch;
