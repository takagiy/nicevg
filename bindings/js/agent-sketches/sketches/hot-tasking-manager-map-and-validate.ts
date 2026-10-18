// HOT Tasking Manager. Sources:
// https://github.com/hotosm/tasking-manager
// https://wiki.openstreetmap.org/wiki/OSM_Tasking_Manager/Validating_data
// The sketch follows a HOT mapping project from its request and project setup through one task's
// editing, OSM upload, independent quality review, correction loop, validation and project export.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "step" | "gate";
type N = SketchNode & { class: Kind; label: string };
const step = ({ label }: N) => {
  const lines = label.split("\n").map((line, i) => text(line, i ? 11 : 12));
  const w = Math.max(...lines.map((line) => line.width)) + 20;
  const h = lines.reduce((sum, line) => sum + line.height, 16);
  let y = 8;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="3" fill="#f8fafc" stroke="#475569"/>${lines.map((line, i) => { const el = line.at({ left: 10, top: y }); y += line.height; return i === 0 ? svg`<rect width="${w}" height="${line.height + 10}" fill="#dbeafe"/>${el}` : el; })}`;
};
const gate = ({ label }: N) => {
  const t = text(label, 12), w = t.width + 34, h = t.height + 28;
  return svg`<path data-bounds="true" d="M ${w / 2} 0 L ${w} ${h / 2} L ${w / 2} ${h} L 0 ${h / 2} Z" fill="#fef3c7" stroke="#92400e"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};

const defs: [string, number, number, string, Kind?][] = [
  ["need",20,70,"Humanitarian organization\nidentifies a mapping need"], ["area",185,55,"Area of interest\ndisaster / service gap"], ["request",355,75,"Request to map\npriority, purpose, deadline"],
  ["creator",525,45,"Project creator\nHOT / partner organization"], ["scope",690,55,"Define project area\npolygon or uploaded boundary"], ["split",850,45,"Split area into task squares"], ["instructions",1015,50,"Write mapping instructions\nfeatures, tags, imagery"], ["imagery",1185,35,"Choose imagery\nsource and date"], ["priority",1185,120,"Set priority and\nproject metadata"],
  ["validationrule",1365,55,"Choose who may validate\nany user or selected teams"], ["publishgate",1535,60,"Ready for mapping?","gate"], ["fixsetup",1535,170,"Revise boundary, imagery\nor instructions"], ["published",1710,60,"Published project\ntasks visible to contributors"],
  ["community",1880,25,"HOT / OSM community\nshare mapping request"], ["mapper",1880,125,"Volunteer mapper\nopens project"], ["read",2045,35,"Read project instructions\nand background"], ["select",2045,140,"Select an available task"], ["lock",2210,125,"Task becomes locked\nfor this mapper"],
  ["editor",2375,45,"Open task in editor\niD or JOSM"], ["imageryload",2375,160,"Load task boundary\nand selected imagery"], ["trace",2540,35,"Trace visible buildings,\nroads and requested features"], ["tags",2540,125,"Add required OSM tags\nper project instructions"], ["connect",2710,45,"Connect roads and ways\nacross the task"], ["selfcheck",2710,155,"Review edits against\ninstructions and imagery"], ["upload",2880,45,"Upload changes to\nOpenStreetMap API"], ["changeset",3045,45,"OSM changeset\nfeatures, tags, comment"], ["mapped",3210,65,"Mark task mapped\nready for validation"], ["release",3210,165,"Task lock released\nfor independent review"],
  ["validator",3380,35,"Validator\nnot the task's mapper"], ["loadreview",3380,130,"Load task square in JOSM\nwith OSM data"], ["readreview",3545,30,"Re-read project instructions\nand local tag guidance"], ["validatorcheck",3545,120,"Run JOSM Validator\ninspect flagged geometry"],
  ["buildings",3710,15,"Check buildings\nshape, tags, completeness"], ["roads",3710,100,"Check highway tags\nand classification"], ["crossings",3710,185,"Check ways connect\nwhere they cross"], ["overlaps",3875,15,"Check overlapping areas\nand tile-edge duplicates"], ["missing",3875,105,"Check missing features\nand task coverage"], ["comment",3875,200,"Prepare constructive\nvalidation comment"], ["qualitygate",4045,95,"Task complete and correct?","gate"],
  ["fixlocal",4045,230,"Validator may fix clear\nsmall errors in JOSM"], ["savefix",4215,230,"Upload validator's\ncorrection changeset"], ["validate",4215,70,"Validate task in\nTasking Manager"], ["feedback",4385,175,"Invalidate task and leave\nfeedback / @mention mapper"], ["tips",4555,175,"Link relevant help\nbuilding / tagging guidance"], ["mapperreturn",4725,105,"Mapper returns to task\nreads validator feedback"], ["unlockagain",4725,200,"Task available for\ncorrection"],
  ["reopen",4895,95,"Reopen task in editor"], ["correct",5065,70,"Correct missing features,\ntags or geometry"], ["uploadagain",5235,70,"Upload corrected\nOSM changeset"], ["remapped",5405,95,"Mark task mapped again"], ["reviewagain",5575,65,"Independent validation\nrepeats review"],
  ["green",4385,25,"Task validated\ngreen status"], ["allmapped",4555,20,"Project reaches\n100% mapped"], ["allvalidated",4725,20,"Project reaches\n100% validated"], ["thirdpass",4895,20,"Third-pass validation\ncheck continuity / omissions"], ["city",5065,20,"Review town-scale\nroad continuity in JOSM"], ["network",5235,20,"Correct missing links,\nclassification and isolation"], ["complete",5405,20,"Project creator confirms\nusable mapped coverage"], ["export",5575,20,"Download OSM export\nfor project area"], ["use",5745,20,"Humanitarian partner\nuses or shares map data"],
  ["osmdata",2880,205,"Existing OpenStreetMap\ndata for task area"], ["apiresult",3045,205,"OSM API accepts upload\nor reports conflict"], ["conflict",3210,250,"Upload conflict /\nversion mismatch","gate"], ["resolve",3380,250,"Refresh OSM data,\nreconcile edits in editor"], ["retryupload",3550,250,"Retry changeset upload"],
];
const nodes: N[] = defs.map(([id,x,y,label,kind]) => ({ id, class: kind ?? "step", x, y, label }));
const links: [string,string,string?][] = [
  ["need","area"],["area","request"],["request","creator"],["creator","scope"],["scope","split"],["split","instructions"],["instructions","imagery"],["instructions","priority"],["priority","validationrule"],["imagery","publishgate"],["validationrule","publishgate"],["publishgate","published","yes"],["publishgate","fixsetup","no: revise"],["fixsetup","scope"],
  ["published","community"],["community","mapper"],["published","mapper"],["mapper","read"],["read","select"],["select","lock"],["lock","editor"],["editor","imageryload"],["imageryload","trace"],["trace","tags"],["tags","connect"],["connect","selfcheck"],["selfcheck","upload"],["upload","osmdata","download current data"],["osmdata","apiresult"],["upload","apiresult"],["apiresult","changeset","accepted"],["apiresult","conflict","conflict"],["conflict","resolve"],["resolve","retryupload"],["retryupload","apiresult","retry"],["changeset","mapped"],["mapped","release"],
  ["release","validator"],["validator","loadreview"],["loadreview","readreview"],["readreview","validatorcheck"],["validatorcheck","buildings"],["validatorcheck","roads"],["validatorcheck","crossings"],["validatorcheck","overlaps"],["validatorcheck","missing"],["buildings","qualitygate"],["roads","qualitygate"],["crossings","qualitygate"],["overlaps","qualitygate"],["missing","qualitygate"],["qualitygate","fixlocal","small clear issue"],["fixlocal","savefix"],["savefix","validate"],["qualitygate","validate","complete"],["qualitygate","feedback","problems remain"],["feedback","tips"],["tips","mapperreturn"],["mapperreturn","unlockagain"],["unlockagain","reopen"],["reopen","correct"],["correct","uploadagain"],["uploadagain","remapped"],["remapped","reviewagain"],["reviewagain","validatorcheck","review again"],["validate","green"],
  ["green","allmapped"],["allmapped","allvalidated"],["allvalidated","thirdpass"],["thirdpass","city"],["city","network"],["network","complete"],["complete","export"],["export","use"],
];
export default { nodes, edges: links.map(([from,to,label]) => ({ from, to, ...(label ? { label } : {}) })), style: { step, gate }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<N>;
