import { useId, Children, useRef, useEffect } from "react";

import { useDraggable, useDroppable } from '@dnd-kit/core';

import { useSpring, animated } from 'react-spring';

import {
    useFloating, offset, flip, shift, autoUpdate
} from '@floating-ui/react';

import { Tooltip, TooltipContent, TooltipTrigger } from "./Tooltip";


export function DragItem(props) {
    const id = useId();

    const { attributes, listeners, setNodeRef, transform } = useDraggable({
        id: id,
        data: { collection: props.collection, index: props.index, type: props.type },
        disabled: props.disabled
    });
    const style = transform ? {
        visibility: "hidden"
    } : {};

    style.userSelect = "none";
    style.msUserSelect = "none";
    style.WebkitUserSelect = "none";

    if (props.disabled) {
        style.cursor = "default";
    }

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
                {
                    props.disabled ?
                    <Tooltip>
                        <TooltipContent>
                            <div className="bg-gray-700 text-white text-xs p-1 rounded bg-opacity-75">
                                {props.disabledMessage}
                            </div>
                        </TooltipContent> 
                        <TooltipTrigger asChild>
                        <div>
                            {props.children}
                        </div>
                        </TooltipTrigger>    
                    </Tooltip>
                    : props.children
                }
                
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
                            <DragItem disabledMessage={props.disabledMessage} disabled={props.disabled} collection={props.collection} index={idx} type={props.type}>{child}</DragItem>
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
                {Children.count(props.children) > 0 ? <DragItem disabledMessage={props.disabledMessage} disabled={props.disabled} className={props.slotClassName} collection={props.collection} type={props.type}>{props.children}</DragItem> : []}
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
        from: { opacity: 0 },
        to: { opacity: simulate_insert ? 1 : 0 },
        config: {
            tension: 210, friction: 20
        }
    });

    const innerStyle = {
        height: "2px",
    };

    return (
        <animated.div style={({
            backgroundColor: "rgb(66, 153, 225)",
            ...style
        })}>
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