// Discourse content records, based on the project's model definitions:
// https://github.com/discourse/discourse/blob/main/app/models/topic.rb
// https://github.com/discourse/discourse/blob/main/app/models/post.rb
// https://github.com/discourse/discourse/blob/main/app/models/post_revision.rb
// https://github.com/discourse/discourse/blob/main/app/models/post_action.rb
// https://github.com/discourse/discourse/blob/main/app/models/topic_user.rb
// https://github.com/discourse/discourse/blob/main/app/models/topic_link.rb
// https://github.com/discourse/discourse/blob/main/app/models/upload_reference.rb
// https://github.com/discourse/discourse/blob/main/app/models/category.rb
// https://github.com/discourse/discourse/blob/main/app/models/tag.rb
// This slice follows forum topics, their ordered posts, and the records that capture edits,
// reactions, reading state, categorization, uploads, bookmarks, and links between posts.
import { svg, text } from "nicevg";
import type { Sketch } from "nicevg";

type Column = [name: string, type: string, key?: "PK" | "FK"];
type Table = { id: string; class: "table"; x: number; y: number; parent: string; columns: Column[] };
type Area = { id: string; class: "area"; x: number; y: number; title: string; width: number; height: number; fill: string };
const FONT = 10, HEAD = 23, ROW = 17, PAD = 6, KEY = 22;
const table = ({ id, columns }: Table) => {
  const title = text(id, 12), names = columns.map(([n]) => text(n, FONT)), types = columns.map(([, t]) => text(t, FONT));
  const width = Math.max(title.width + PAD * 2, KEY + Math.max(...names.map(n => n.width)) + Math.max(...types.map(t => t.width)) + PAD * 3);
  const height = HEAD + columns.length * ROW, cy = (i: number) => HEAD + i * ROW + ROW / 2;
  return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="white" stroke="#475569"/><rect width="${width}" height="${HEAD}" fill="#e2e8f0" stroke="#475569"/>${title.at({ cx: width / 2, cy: HEAD / 2 })}${columns.map(([n,,k],i) => [k ? text(k,9).at({left:PAD,cy:cy(i)}) : "", names[i]!.at({left:KEY,cy:cy(i)}), types[i]!.at({right:width-PAD,cy:cy(i)}), [0,width].map(x => svg`<circle data-port="${n}" cx="${x}" cy="${cy(i)}" r="0"/>`)])}`;
};
const area = ({ title, width, height, fill }: Area) => svg`<rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#94a3b8"/>${text(title,13).at({left:10,top:7})}`;
const tables: Omit<Table,"class">[] = [
 {id:"users",parent:"community",x:35,y:55,columns:[["id","int","PK"],["username","varchar"],["name","varchar"],["trust_level","int"]]},
 {id:"categories",parent:"community",x:35,y:200,columns:[["id","int","PK"],["name","varchar"],["parent_category_id","int","FK"],["topic_count","int"]]},
 {id:"category_users",parent:"community",x:300,y:210,columns:[["id","int","PK"],["category_id","int","FK"],["user_id","int","FK"],["notification_level","int"]]},
 {id:"topics",parent:"discussion",x:640,y:55,columns:[["id","int","PK"],["user_id","int","FK"],["category_id","int","FK"],["title","varchar"],["archetype","varchar"],["posts_count","int"],["last_posted_at","datetime"]]},
 {id:"posts",parent:"discussion",x:950,y:55,columns:[["id","int","PK"],["topic_id","int","FK"],["user_id","int","FK"],["reply_to_post_number","int"],["post_number","int"],["raw","text"],["cooked","text"],["deleted_at","datetime"]]},
 {id:"post_revisions",parent:"discussion",x:950,y:260,columns:[["id","int","PK"],["post_id","int","FK"],["user_id","int","FK"],["number","int"],["modifications","jsonb"],["hidden","boolean"]]},
 {id:"post_actions",parent:"engagement",x:1300,y:55,columns:[["id","int","PK"],["post_id","int","FK"],["user_id","int","FK"],["post_action_type_id","int"],["deleted_at","datetime"]]},
 {id:"post_action_types",parent:"engagement",x:1570,y:60,columns:[["id","int","PK"],["name_key","varchar"],["is_like","boolean"]]},
 {id:"bookmarks",parent:"engagement",x:1300,y:205,columns:[["id","int","PK"],["user_id","int","FK"],["bookmarkable_id","int"],["bookmarkable_type","varchar"],["name","varchar"]]},
 {id:"topic_users",parent:"engagement",x:1570,y:220,columns:[["id","int","PK"],["topic_id","int","FK"],["user_id","int","FK"],["last_read_post_number","int"],["notification_level","int"],["bookmarked","boolean"]]},
 {id:"post_timings",parent:"engagement",x:1300,y:395,columns:[["id","int","PK"],["topic_id","int","FK"],["user_id","int","FK"],["post_number","int"],["msecs","int"]]},
 {id:"topic_links",parent:"references",x:950,y:470,columns:[["id","int","PK"],["post_id","int","FK"],["topic_id","int","FK"],["link_post_id","int","FK"],["link_topic_id","int","FK"],["url","varchar"],["reflection","boolean"]]},
 {id:"uploads",parent:"references",x:1300,y:610,columns:[["id","int","PK"],["user_id","int","FK"],["original_filename","varchar"],["filesize","bigint"],["url","varchar"]]},
 {id:"upload_references",parent:"references",x:1570,y:610,columns:[["id","bigint","PK"],["upload_id","bigint","FK"],["target_type","varchar"],["target_id","bigint"]]},
 {id:"tags",parent:"community",x:35,y:415,columns:[["id","int","PK"],["name","varchar"],["slug","varchar"]]},
 {id:"topic_tags",parent:"community",x:300,y:415,columns:[["id","int","PK"],["topic_id","int","FK"],["tag_id","int","FK"]]},
 {id:"post_stats",parent:"discussion",x:640,y:300,columns:[["id","int","PK"],["post_id","int","FK"],["reply_count","int"],["like_count","int"],["incoming_link_count","int"]]},
 {id:"post_replies",parent:"discussion",x:640,y:475,columns:[["id","int","PK"],["post_id","int","FK"],["reply_id","int","FK"]]},
 {id:"topic_allowed_users",parent:"community",x:35,y:585,columns:[["id","int","PK"],["topic_id","int","FK"],["user_id","int","FK"]]},
 {id:"topic_allowed_groups",parent:"community",x:300,y:585,columns:[["id","int","PK"],["topic_id","int","FK"],["group_id","int","FK"]]},
 {id:"groups",parent:"community",x:35,y:740,columns:[["id","int","PK"],["name","varchar"],["visibility_level","int"]]},
 {id:"group_users",parent:"community",x:300,y:740,columns:[["id","int","PK"],["group_id","int","FK"],["user_id","int","FK"]]},
 {id:"post_custom_fields",parent:"discussion",x:640,y:650,columns:[["id","int","PK"],["post_id","int","FK"],["name","varchar"],["value","text"]]},
];
const areas: Area[] = [
 {id:"community",class:"area",x:15,y:25,width:555,height:900,title:"People, categories, tags and access",fill:"#f0f9ff"},
 {id:"discussion",class:"area",x:615,y:25,width:560,height:790,title:"Topics and post content",fill:"#f0fdf4"},
 {id:"engagement",class:"area",x:1275,y:25,width:560,height:535,title:"Reactions and per-user topic state",fill:"#fdf4ff"},
 {id:"references",class:"area",x:1275,y:585,width:560,height:290,title:"Attachments and external/internal links",fill:"#fff7ed"},
];
const nodes: (Table|Area)[] = [...areas,...tables.map(t=>({...t,class:"table" as const}))];
const pairs = [
 ["topics.user_id","users.id"],["topics.category_id","categories.id"],["categories.parent_category_id","categories.id"],
 ["category_users.category_id","categories.id"],["category_users.user_id","users.id"],
 ["posts.topic_id","topics.id"],["posts.user_id","users.id"],["post_revisions.post_id","posts.id"],["post_revisions.user_id","users.id"],
 ["post_actions.post_id","posts.id"],["post_actions.user_id","users.id"],["post_actions.post_action_type_id","post_action_types.id"],
 ["bookmarks.user_id","users.id"],["topic_users.topic_id","topics.id"],["topic_users.user_id","users.id"],
 ["post_timings.topic_id","topics.id"],["post_timings.user_id","users.id"],
 ["topic_links.post_id","posts.id"],["topic_links.topic_id","topics.id"],["topic_links.link_post_id","posts.id"],["topic_links.link_topic_id","topics.id"],
 ["uploads.user_id","users.id"],["upload_references.upload_id","uploads.id"],
 ["topic_tags.topic_id","topics.id"],["topic_tags.tag_id","tags.id"],["post_stats.post_id","posts.id"],
 ["post_replies.post_id","posts.id"],["post_replies.reply_id","posts.id"],
 ["topic_allowed_users.topic_id","topics.id"],["topic_allowed_users.user_id","users.id"],["topic_allowed_groups.topic_id","topics.id"],["topic_allowed_groups.group_id","groups.id"],
 ["group_users.group_id","groups.id"],["group_users.user_id","users.id"],["post_custom_fields.post_id","posts.id"],
];
const edges = pairs.map(([from,to])=>({from:from!,to:to!}));
const sketch = {nodes,edges,style:{table,area},rootAttrs:{"font-family":"sans-serif"}} satisfies Sketch<Table|Area>;
export default sketch;
