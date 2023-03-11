import { useState, useId, useMemo } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import { useDraggable, useDroppable } from '@dnd-kit/core';
import { CSS } from '@dnd-kit/utilities';



function DragItem(props) {
    const id = useId();

    const { attributes, listeners, setNodeRef, transform } = useDraggable({
        id: id,
        data: { collection: props.collection, index: props.index, type: props.type }
    });
    const style = transform ? {
        transform: CSS.Translate.toString(transform),
    } : undefined;


    return (
        <div ref={setNodeRef} style={style} {...listeners} {...attributes}>
            {props.children}
        </div>
    );
}


function DropSlot(props) {
    const id = useId();

    const { isOver, setNodeRef } = useDroppable({
        id: id,
        data: { collection: props.collection, index: props.index, isPlaceholder: false, type: props.type }
    });

    const style = {
    }

    return (
        <div ref={setNodeRef} style={style}>
            {props.children}
        </div>
    );
}


export function DropList(props) {
    return (
        <div>
            <Placeholder collection={props.collection} index={0} type={props.type} />
            {props.children.flatMap(
                (child, idx) => [
                    <DropSlot key={`item_${idx}`} collection={props.collection} index={idx} type={props.type}>
                        <DragItem collection={props.collection} index={idx} type={props.type}>{child}</DragItem>
                    </DropSlot>,
                    <Placeholder key={`placeholder_${idx}`} collection={props.collection} index={idx + 1} type={props.type} />])}
        </div>
    );
}


export function DropWell(props) {
    return (
        <div>
            <DropSlot collection={props.collection} type={props.type}>
                <DragItem collection={props.collection} type={props.type}>{props.children}</DragItem>
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

    const style = {
        height: "10px",
    };
    const container_style = {
        height: simulate_insert ? `25px` : "auto",
    };

    return (
        <div style={container_style}>
            <div ref={setNodeRef} style={style}>
            </div>
        </div>
    );
}


export function makeDragHandler(f) {
    return (event) => {
        console.log(event.active.data.current, event.over.data.current);
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