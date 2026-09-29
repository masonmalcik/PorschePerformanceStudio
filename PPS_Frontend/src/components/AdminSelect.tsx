import { useEffect, useRef, useState } from 'react';

export type SelectOption = { value: string; label: string };
type Props = { value: string; options: SelectOption[]; placeholder: string; disabled?: boolean; required?: boolean; onChange: (value: string) => void };

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

export default function AdminSelect({ value, options, placeholder, disabled=false, required=true, onChange }: Props) {
  const [open,setOpen]=useState(false),[invalid,setInvalid]=useState(false); const root=useRef<HTMLDivElement>(null);
  useEffect(()=>{const close=(event:MouseEvent)=>{if(root.current&&!root.current.contains(event.target as Node))setOpen(false)};document.addEventListener('mousedown',close);return()=>document.removeEventListener('mousedown',close)},[]);
  useEffect(()=>{
    const node=root.current,form=node?.closest('form');
    if(!node||!form)return;
    form.noValidate=true;
    const isOptional=(control:HTMLElement)=>control.closest('label')?.textContent?.toLowerCase().includes('(optional)')??false;
    const validateSelf=()=>setInvalid(required&&!isOptional(node)&&!value);
    const validateAll=(event:SubmitEvent)=>{
      const selects=Array.from(form.querySelectorAll<HTMLElement>('[data-admin-select-required="true"]')).filter(control=>!isOptional(control));
      selects.forEach(control=>control.dispatchEvent(new Event('admin-select-validate')));
      const nativeFields=Array.from(form.querySelectorAll<HTMLInputElement|HTMLTextAreaElement>('input[required],textarea[required]')).filter(control=>!control.disabled&&control.type!=='hidden');
      const emptyFields=nativeFields.filter(control=>!control.value.trim());
      emptyFields.forEach(control=>{
        const label=control.closest('label');
        if(!label)return;
        label.classList.add('form-field','invalid');
        if(!label.querySelector('[data-native-field-error]')){
          const message=document.createElement('small');
          message.className='field-error';
          message.dataset.nativeFieldError='true';
          message.textContent='This Field is Required';
          label.append(message);
        }
        if(!control.dataset.adminValidationBound){
          control.dataset.adminValidationBound='true';
          control.addEventListener('input',()=>{
            if(!control.value.trim())return;
            label.classList.remove('invalid');
            label.querySelector('[data-native-field-error]')?.remove();
          });
        }
      });
      if(selects.some(control=>!control.dataset.adminSelectValue)||emptyFields.length){
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    };
    node.addEventListener('admin-select-validate',validateSelf);
    form.addEventListener('submit',validateAll,true);
    return()=>{node.removeEventListener('admin-select-validate',validateSelf);form.removeEventListener('submit',validateAll,true)};
  },[required,value]);
  const selected=value ? options.find((option)=>option.value===value) : undefined;
  return <div className={`model-select${open?' open':''}${invalid?' invalid':''}`} data-admin-select-required={required?'true':'false'} data-admin-select-value={value} ref={root}>
    <button className="model-select-trigger" type="button" aria-haspopup="listbox" aria-expanded={open} disabled={disabled} onClick={()=>setOpen((current)=>!current)} onKeyDown={(event)=>{if(event.key==='Escape')setOpen(false)}}>
      <span className={selected?'selected':'placeholder'}>{selected?.label??titleCasePlaceholder(placeholder)}</span><span className="model-select-chevron" aria-hidden="true"></span>
    </button>
    {open&&<div className="model-select-options" role="listbox">{options.map((option)=><button className={option.value===value?'selected':''} type="button" role="option" aria-selected={option.value===value} key={option.value} onClick={()=>{setInvalid(false);onChange(option.value);setOpen(false)}}>{option.label}</button>)}</div>}
    {invalid&&<small className="field-error">This Field is Required</small>}
  </div>;
}
