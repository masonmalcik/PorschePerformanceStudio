import { useEffect, useRef, useState } from 'react';

export type SelectOption = { value: string; label: string };
type Props = { value: string; options: SelectOption[]; placeholder: string; disabled?: boolean; onChange: (value: string) => void };

function titleCasePlaceholder(value: string): string {
  const minorWords = new Set(['a', 'an', 'and', 'as', 'at', 'by', 'for', 'in', 'of', 'on', 'or', 'the', 'to']);
  const words = value.match(/[A-Za-z]+/g) ?? [];
  let index = 0;
  return value.replace(/[A-Za-z]+/g, (word) => {
    const lower = word.toLowerCase();
    const isMinor = index > 0 && index < words.length - 1 && minorWords.has(lower);
    index += 1;
    return isMinor ? lower : lower[0].toUpperCase() + lower.slice(1);
  });
}

export default function AdminSelect({ value, options, placeholder, disabled=false, onChange }: Props) {
  const [open,setOpen]=useState(false); const root=useRef<HTMLDivElement>(null);
  useEffect(()=>{const close=(event:MouseEvent)=>{if(root.current&&!root.current.contains(event.target as Node))setOpen(false)};document.addEventListener('mousedown',close);return()=>document.removeEventListener('mousedown',close)},[]);
  const selected=value ? options.find((option)=>option.value===value) : undefined;
  return <div className={`model-select${open?' open':''}`} ref={root}>
    <button className="model-select-trigger" type="button" aria-haspopup="listbox" aria-expanded={open} disabled={disabled} onClick={()=>setOpen((current)=>!current)} onKeyDown={(event)=>{if(event.key==='Escape')setOpen(false)}}>
      <span className={selected?'selected':'placeholder'}>{selected?.label??titleCasePlaceholder(placeholder)}</span><span className="model-select-chevron" aria-hidden="true"></span>
    </button>
    {open&&<div className="model-select-options" role="listbox">{options.map((option)=><button className={option.value===value?'selected':''} type="button" role="option" aria-selected={option.value===value} key={option.value} onClick={()=>{onChange(option.value);setOpen(false)}}>{option.label}</button>)}</div>}
  </div>;
}
