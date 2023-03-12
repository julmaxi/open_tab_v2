import { useState, useId, useMemo, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DropList, DropWell, makeDragHandler} from './DragDrop.jsx';


function TeamItem(props) {
  return <div>
    <div>{props.team.name}</div>
  </div>
}


function SpeakerItem(props) {
  return <div>
    <div>{props.speaker.name}</div>
  </div>
}


function AdjudicatorItem(props) {
  return <div>
    <div>{props.adjudicator.name}</div>
  </div>
}

function DebateRow(props) {
  return (
    <tr>
      <td>
        <DropWell type="team" collection={[props.debate.index, "government"]}><TeamItem team={props.debate.government} /></DropWell>
        <br />
        <DropWell type="team" collection={[props.debate.index, "opposition"]}><TeamItem team={props.debate.opposition} /></DropWell>
      </td>
      <td>
        <DropList type="speaker" collection={[props.debate.index, "non_aligned_speakers"]}>
          {props.debate.non_aligned_speakers.map((speaker) => <SpeakerItem key={speaker.uuid} speaker={speaker} />)}
        </DropList>
      </td>
      <td>
        <DropList type="adjudicator" collection={[props.debate.index, "adjudicators"]}>
          {props.debate.adjudicators.map((adjudicator) => <AdjudicatorItem key={adjudicator.uuid} adjudicator={adjudicator} />)}
        </DropList>
      </td>
      <td>
        <DropWell type="adjudicator" collection={[props.debate.index, "president"]}>{props.debate.president ? <AdjudicatorItem adjudicator={props.debate.president} /> : []}</DropWell>
      </td>
    </tr>
  );
}


function getPath(obj, path) {
  return path.reduce((acc, part) => acc[part], obj);
}


function clone(e) {
  return structuredClone(e);
}

function updatePath(obj, path, new_val) {
  if (path.length == 0) {
    return new_val;
  }
  let child = obj[path[0]];

  let val = updatePath(child, path.slice(1), new_val)
  obj[path[0]] = val;

  return obj;
}

function simulateDragOutcome(draw, from, to, isSwap) {
  var from_debate = clone(draw[from.collection[0]]);

  var to_debate;
  if (from.collection[0] == to.collection[0]) {
    to_debate = from_debate;
  }
  else {
    to_debate = clone(draw[to.collection[0]]);
  }

  var from_collection = getPath(from_debate, from.collection.slice(1));
  var to_collection;

  if (from.collection == to.collection) {
    to_collection = from_collection
  }
  else {
    to_collection = getPath(to_debate, to.collection.slice(1));
  }

  if (to.index !== undefined && from.index !== undefined) {
    if (isSwap) {
      let tmp = from_collection[from.index];
      from_collection[from.index] = to_collection[to.index];
      to_collection[to.index] = tmp;
    }
    else {
      if (from.index < to.index) {
        let tmp = from_collection[from.index];
        to_collection.splice(to.index, 0, tmp);
        from_collection.splice(from.index, 1);  
      }
      else {
        let tmp = from_collection[from.index];
        from_collection.splice(from.index, 1);
        to_collection.splice(to.index, 0, tmp);
      }
    }
  } else if (to.index !== undefined) {
    let from_val = from_collection;
    let to_val = to_collection[to.index];
    from_collection = isSwap ? to_val : null;
    to_collection.splice(to.index, isSwap ? 1 : 0, from_val);
  } else if (from.index !== undefined) {
    let from_val = from_collection[from.index];
    let to_val = to_collection;
    if (isSwap && to_val !== null) {
      from_collection.splice(from.index, 1, to_val);
    }
    else {
      from_collection.splice(from.index, 1);
    }
    console.log(from_collection);
    to_collection = from_val;
  } else {
    let tmp = from_collection;
    from_collection = to_collection;
    to_collection = tmp;
  }

  updatePath(from_debate, from.collection.slice(1), from_collection);
  updatePath(to_debate, to.collection.slice(1), to_collection);

  if (from.collection[0] == to.collection[0]) {
    return {[from.collection[0]]: from_debate};
  }
  else {
    return {[from.collection[0]]: from_debate, [to.collection[0]]: to_debate};
  }
}

function DrawEditor() {
  const [draw, setDraw] = useState([]);

  function onDragEnd(from, to, isSwap) {
    let changedRooms = simulateDragOutcome(draw, from, to, isSwap);

    let newDraw = structuredClone(draw);

    for (let [index, debate] of Object.entries(changedRooms)) {
      newDraw[index] = debate;
    }

    setDraw(newDraw);
  }

  useEffect(() => {
    invoke("subscribe_to_view", {view: {type: "Draw", uuid: 1}}).then((msg) => {
      let draw = JSON.parse(msg);
      setDraw(draw.debates);
    });
  }, []);

  let dragEnd = useCallback(makeDragHandler(onDragEnd), [draw]);

  return <div>
    <DndContext collisionDetection={closestCenter} onDragEnd={dragEnd}>
      <table>
        <tbody>
          {draw.map((debate) => <DebateRow key={debate.uuid} debate={debate} />)}
        </tbody>
      </table>
    </DndContext>
  </div>
}


export default DrawEditor;
