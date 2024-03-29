import { useState, useId, useMemo, Children } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import { useDraggable, useDroppable } from '@dnd-kit/core';
import { CSS } from '@dnd-kit/utilities';

import { useSpring, animated } from 'react-spring';


export function DragItem(props) {
    const id = useId();

    const { attributes, listeners, setNodeRef, transform } = useDraggable({
        id: id,
        data: { collection: props.collection, index: props.index, type: props.type }
    });
    const style = transform ? {
        visibility: "hidden"
        //transform: CSS.Translate.toString(transform),
    } : undefined;


    if (props.content_tag == "tr") {
        return (
            <tr ref={setNodeRef} style={style} className={props.className || ""} {...listeners} {...attributes}>
                {props.children}
            </tr>
        );    
    }
    else {
        return (
            <div ref={setNodeRef} style={style} className={props.className || ""} {...listeners} {...attributes}>
                {props.children}
            </div>
        );    
    }
}


export function DropSlot(props) {
    const id = useId();

    const { isOver, setNodeRef } = useDroppable({
        id: id,
        data: { collection: props.collection, index: props.index, isPlaceholder: false, type: props.type }
    });

    const style = {
    }

    return (
        <div ref={setNodeRef} style={style} className={props.className || ""}>
            {props.children}
        </div>
    );
}


export function DropList(props) {
    return (
        <div style={{minWidth: props.minWidth}}>
            <Placeholder collection={props.collection} index={0} type={props.type} />
            {
                props.children.flatMap(
                    (child, idx) => [
                        <DropSlot key={`item_${idx}`} collection={props.collection} index={idx} type={props.type}>
                            <DragItem collection={props.collection} index={idx} type={props.type}>{child}</DragItem>
                        </DropSlot>,
                        <Placeholder key={`placeholder_${idx}`} collection={props.collection} index={idx + 1} type={props.type} />
                    ]
                )
            }
        </div>
    );
}


export function DropWell(props) {
    return (
        <div style={{minWidth: props.minWidth}} className={props.className || ""}>
            <DropSlot collection={props.collection} type={props.type} className={props.slotClassName}>
                {Children.count(props.children) > 0 ? <DragItem className={props.slotClassName} collection={props.collection} type={props.type}>{props.children}</DragItem> : []}
            </DropSlot>
        </div>
    );
}



function Placeholder(props) {
    const id = useId();

    const { isOver, rect, active, setNodeRef } = useDroppable({
        id: id,
        data: { collection: props.collection, index: props.index, isPlaceholder: true, type: props.type }
    });

    let simulate_insert = isOver && active.data.current.type == props.type;

    let style = useSpring({
        from: { height: 2 },
        to: { height: simulate_insert ? 25 : 2 },
        config: {
            tension: 210, friction: 20
        }
    });


    const innerStyle = {
        height: "2px",
    };

    return (
        <animated.div style={style}>
            <div ref={setNodeRef} style={innerStyle}>
            </div>
        </animated.div>
    );
}


export function makeDragHandler(f) {
    return (event) => {
        if (event.active.data.current.type != event.over.data.current.type) {
            return;
        }

        let from_collection = event.active.data.current.collection;
        let to_collection = event.over.data.current.collection;
      
        let from_index = event.active.data.current.index;
        let to_index = event.over.data.current.index;
        let swap = !event.over.data.current.isPlaceholder;

        f({collection: from_collection, index: from_index}, {collection: to_collection, index: to_index}, swap);
    }
}