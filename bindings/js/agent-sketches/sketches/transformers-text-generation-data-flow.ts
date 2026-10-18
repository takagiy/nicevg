// Hugging Face Transformers text-generation pipeline.
// Sources: https://github.com/huggingface/transformers/blob/main/src/transformers/pipelines/text_generation.py
// and https://github.com/huggingface/transformers/blob/main/docs/source/en/pipeline_tutorial.md
// Follows a prompt through TextGenerationPipeline preprocessing, causal generation,
// and postprocessing into the `generated_text` result.

import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
  parent?: string;
};

const sketch = {
  nodes: [
    { id: "prompt", class: "box", x: 30, y: 180, label: "text prompt\n`str`" },
    { id: "pipeline", class: "box", x: 230, y: 150, label: "TextGenerationPipeline\n__call__(text_inputs, **kwargs)" , width: 225, height: 76},
    { id: "preprocess", class: "box", x: 510, y: 150, label: "preprocess(prompt_text)\napply chat template if chat" , width: 210, height: 76},
    { id: "tokenizer", class: "box", x: 780, y: 150, label: "tokenizer\ninput_ids, attention_mask" , width: 200, height: 76},
    { id: "model", class: "box", x: 1040, y: 120, label: "causal language model\nmodel.generate(**inputs)" , width: 220, height: 76},
    { id: "next-token", class: "box", x: 1320, y: 120, label: "next-token scores\nlogits for final position" , width: 220, height: 76},
    { id: "select", class: "box", x: 1320, y: 270, label: "generation strategy\nselect / sample next token" , width: 220, height: 76},
    { id: "sequence", class: "box", x: 1040, y: 300, label: "generated token sequence\ninput + new token IDs" , width: 220, height: 76},
    { id: "decode", class: "box", x: 780, y: 330, label: "tokenizer.decode(...)\nskip_special_tokens" , width: 200, height: 76},
    { id: "postprocess", class: "box", x: 510, y: 330, label: "postprocess(model_outputs)\nfull text or new text" , width: 210, height: 76},
    { id: "result", class: "box", x: 230, y: 330, label: "list[dict]\n{ generated_text: str }" , width: 225, height: 76},
    { id: "caller", class: "box", x: 30, y: 330, label: "caller receives\nresult list" },
  ] satisfies Node[],
  edges: [
    { from: "prompt", to: "pipeline", label: "text_inputs" },
    { from: "pipeline", to: "preprocess", label: "preprocess stage" },
    { from: "preprocess", to: "tokenizer", label: "prompt / chat-formatted text" },
    { from: "tokenizer", to: "model", label: "model_inputs tensors" },
    { from: "model", to: "next-token", label: "forward pass: logits" },
    { from: "next-token", to: "select", label: "scores + generation config" },
    { from: "select", to: "sequence", label: "selected token ID" },
    { from: "sequence", to: "model", label: "append token; continue until stopping criteria" },
    { from: "sequence", to: "decode", label: "sequences tensor" },
    { from: "decode", to: "postprocess", label: "decoded text" },
    { from: "postprocess", to: "result", label: "generated_text" },
    { from: "result", to: "caller", label: "return" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
