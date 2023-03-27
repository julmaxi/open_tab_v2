import { useState, useId, useMemo, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'
import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DropList, DropWell, makeDragHandler} from './DragDrop.jsx';

import { useView, updatePath, getPath, clone } from './View.js'

function TeamItem(props) {
  return <div>
    <div className="bg-gray-100 min-w-[14rem] rounded-md">
      {props.team.name}
    </div>
  </div>
}


function SpeakerItem(props) {
  return <div className="bg-gray-100 min-w-[14rem] rounded-md">
    <div>{props.speaker.name}</div>
    <div>{props.speaker.team_name}</div>
  </div>
}


function AdjudicatorItem(props) {
  return <div className="bg-gray-100 min-w-[14rem] rounded-md m-0">
    <div>{props.adjudicator.name}</div>
  </div>
}

function DebateRow(props) {
  let ballot = props.debate.ballot;
  return (
    <tr>
      <td>
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "government"]}>
          {ballot.government !== null ? <TeamItem team={ballot.government} /> : []}
        </DropWell>
        <br />
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "opposition"]}>
          {ballot.opposition !== null ? <TeamItem team={ballot.opposition} /> : []}
        </DropWell>
      </td>
      <td>
        <DropList type="speaker" collection={["debates", props.debate.index, "ballot", "non_aligned_speakers"]}>
          {ballot.non_aligned_speakers.map((speaker) => <SpeakerItem key={speaker.uuid} speaker={speaker} />)}
        </DropList>
      </td>
      <td>
        <DropList type="adjudicator" collection={["debates", props.debate.index, "ballot", "adjudicators"]}>
          {ballot.adjudicators.map((adjudicator) => <AdjudicatorItem key={adjudicator.uuid} adjudicator={adjudicator} />)}
        </DropList>
      </td>
      <td>
        <DropWell type="adjudicator" collection={["debates", props.debate.index, "ballot", "president"]}>{ballot.president ? <AdjudicatorItem adjudicator={ballot.president} /> : []}</DropWell>
      </td>
    </tr>
  );
}

function simulateDragOutcome(draw, from, to, isSwap) {
  var from_debate = clone(draw.debates[from.collection[1]]);

  var to_debate;
  if (from.collection[1] == to.collection[1]) {
    to_debate = from_debate;
  }
  else {
    to_debate = clone(draw.debates[to.collection[1]]);
  }

  var from_collection = getPath(draw, from.collection);
  var to_collection;

  if (from.collection == to.collection) {
    to_collection = from_collection
  }
  else {
    to_collection = getPath(draw, to.collection);
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
    to_collection = from_val;
  } else {
    let tmp = from_collection;
    from_collection = to_collection;
    to_collection = tmp;
  }

  updatePath(from_debate, from.collection.slice(2), from_collection);
  updatePath(to_debate, to.collection.slice(2), to_collection);

  if (from.collection[1] == to.collection[1]) {
    return {[from.collection[1]]: from_debate};
  }
  else {
    return {[from.collection[1]]: from_debate, [to.collection[0]]: to_debate};
  }
}

function DrawEditor(props) {
  //const [draw, setDraw] = useState([]);

  function onDragEnd(from, to, isSwap) {
    let changedDebates = simulateDragOutcome(draw, from, to, isSwap);

    executeAction("execute_action", {
        updated_ballots: Object.keys(changedDebates).map(key => changedDebates[key].ballot)
    });
  }

  let currentView = {type: "Draw", uuid: props.round_uuid};
  let draw = useView(currentView, {"debates": []});
  let debates = draw.debates;

  let dragEnd = useCallback(makeDragHandler(onDragEnd), [draw]);

  return <div>
    <DndContext collisionDetection={closestCenter} onDragEnd={dragEnd}>
      <table>
        <tbody>
          {debates.map((debate) => <DebateRow key={debate.uuid} debate={debate} />)}
        </tbody>
      </table>
    </DndContext>
  </div>
  }


export default DrawEditor;
