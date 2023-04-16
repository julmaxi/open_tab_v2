//@ts-check
import React, { useState, useId, useMemo, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'
import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DropList, DropWell, makeDragHandler} from './DragDrop.jsx';

import { useView, updatePath, getPath, clone } from './View.js'
import { executeAction } from "./Action";


function DragBox(props) {  
  return <div className={`flex bg-gray-100 min-w-[14rem] p-1 rounded border ${props.highlightedIssues?.length > 0 ? "border-red-500" : "border-gray-100"}`}>
    <div className="flex-1">
      {props.children}
    </div>
    <div className="flex items-center mr-1">
        <ClashIndicator issues={props.issues.filter(i => i.is_active)} onHover={props.onHighlightIssues} />
    </div>
  </div>
}


function HorizontalList(props) {
  return <div className="flex flex-row gap-x-1">
    {props.children}
  </div>
}

function TeamItem(props) {
  let all_participant_institutions = props.team.members.map((m) => m.institutions).flat().sort((a, b) => a.name.localeCompare(b.name));
  let unique_participant_institutions = [...new Set(all_participant_institutions.map((i) => i.uuid))].map((uuid) => all_participant_institutions.find((i) => i.uuid === uuid));

  let all_participant_issues = props.team.members.map((m) => m.issues).flat();
  let highlightedIssues = all_participant_issues.filter((i) => props.issueHightlightedParticipantUuids.includes(i.target_participant_id));

  return <DragBox issues={all_participant_issues} onHighlightIssues={(shouldHighlight) => {
    if (shouldHighlight) {
      props.onAddIssueHighlightUuids(props.team.members.map((m) => m.uuid));
    }
    else {
      props.onRemoveIssueHighlightUuids(props.team.members.map((m) => m.uuid));
    }
  }} highlightedIssues={highlightedIssues}>
      <div>{props.team.name}</div>
      <HorizontalList>
        {props.team.members.map((member) => <div key={member.uuid} className="text-xs">{member.name}</div>)}
      </HorizontalList>
      <HorizontalList>
        {unique_participant_institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}


function ClashIndicator(props) {
  let issueBuckets = props.issues.reduce((acc, issue) => {
    if (issue.severity >= 75) {
      acc.high.push(issue);
    } else if (issue.severity >= 50) {
      acc.mid.push(issue);
    } else if (issue.severity >= 25) {
      acc.low.push(issue);
    } else {
      acc.misc.push(issue);
    }
    return acc;
  }, {misc: [], low: [], mid: [], high: []});

  const issueColors = {
    misc: "bg-gray-500",
    low: "bg-blue-500",
    mid: "bg-yellow-500",
    high: "bg-red-500"
  }

  return <div className="flex h-6 rounded-md overflow-hidden w-12" onMouseEnter={() => props.onHover(true)} onMouseLeave={() => props.onHover(false)}>
    {
      props.issues.length == 0 ?
      <div className="h-full flex-1 items-center bg-green-500 text-white text-sm pl-1 pr-1">{"\u2713"}</div>
      :
      ["misc", "low", "mid", "high"].map(
        (key) => {
          return issueBuckets[key].length > 0 ?
          <div key={key} className={`h-full flex flex-1 items-center text-white text-sm pl-1 pr-1 ${issueColors[key]}`}>{issueBuckets[key].length}</div>
          :
          null
        }
      )
    }
  </div>;
}


function SpeakerItem(props) {
  let highlightedIssues = props.speaker.issues.filter((i) => props.issueHightlightedParticipantUuids.includes(i.target_participant_id));

  return <DragBox issues={props.speaker.issues} onHighlightIssues={(shouldHighlight) => {
    if (shouldHighlight) {
      props.onAddIssueHighlightUuids([props.speaker.uuid]);
    }
    else {
      props.onRemoveIssueHighlightUuids([props.speaker.uuid]);
    }
  }} highlightedIssues={highlightedIssues}>
    <div>{props.speaker.name}</div>
    <div className="text-xs">{props.speaker.team_name}</div>
    <HorizontalList>
      {props.speaker.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}


function AdjudicatorItem(props) {
  let highlightedIssues = props.adjudicator.issues.filter((i) => props.issueHightlightedParticipantUuids.includes(i.target_participant_id));

  return <DragBox issues={props.adjudicator.issues} onHighlightIssues={(shouldHighlight) => {
    if (shouldHighlight) {
      props.onAddIssueHighlightUuids([props.adjudicator.uuid]);
    }
    else {
      props.onRemoveIssueHighlightUuids([props.adjudicator.uuid]);
    }
  }} highlightedIssues={highlightedIssues}>
    <div>{props.adjudicator.name}</div>
    <HorizontalList>
      {props.adjudicator.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}

function DebateRow(props) {
  let ballot = props.debate.ballot;
  let [issueHightlightedParticipantUuids, setIssueHightlightedParticipantUuids] = useState([]);

  return (
    <tr>
      <td>
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "government"]}>
          {ballot.government !== null ? <TeamItem
            team={ballot.government}
            onAddIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids([...issueHightlightedParticipantUuids, ...uuids])
            }
            onRemoveIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids(issueHightlightedParticipantUuids.filter((uuid) => !uuids.includes(uuid)))
            }
            issueHightlightedParticipantUuids={issueHightlightedParticipantUuids}
            /> : []}
        </DropWell>
        <br />
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "opposition"]}>
          {ballot.opposition !== null ? <TeamItem
            team={ballot.opposition}
            onAddIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids([...issueHightlightedParticipantUuids, ...uuids])
            }
            onRemoveIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids(issueHightlightedParticipantUuids.filter((uuid) => !uuids.includes(uuid)))
            }
            issueHightlightedParticipantUuids={issueHightlightedParticipantUuids}            
            /> : []}
        </DropWell>
      </td>
      <td>
        <DropList type="speaker" collection={["debates", props.debate.index, "ballot", "non_aligned_speakers"]}>
          {ballot.non_aligned_speakers.map((speaker) =>
            <SpeakerItem
            key={speaker.uuid}
            speaker={speaker}
            onAddIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids([...issueHightlightedParticipantUuids, ...uuids])
            }
            onRemoveIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids(issueHightlightedParticipantUuids.filter((uuid) => !uuids.includes(uuid)))
            }
            issueHightlightedParticipantUuids={issueHightlightedParticipantUuids}
            />)}
        </DropList>
      </td>
      <td>
        <DropList type="adjudicator" collection={["debates", props.debate.index, "ballot", "adjudicators"]}>
          {ballot.adjudicators.map((adjudicator) =>
            <AdjudicatorItem
            key={adjudicator.uuid}
            adjudicator={adjudicator}
            onAddIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids([...issueHightlightedParticipantUuids, ...uuids])
            }
            onRemoveIssueHighlightUuids={
              (uuids) => setIssueHightlightedParticipantUuids(issueHightlightedParticipantUuids.filter((uuid) => !uuids.includes(uuid)))
            }
            issueHightlightedParticipantUuids={issueHightlightedParticipantUuids}
          />)}
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


function TabGroup(props) {
  let [activeTab, setActiveTab] = useState(0);

  let children = React.Children.toArray(props.children);
  let displayChild = children[activeTab];

  return <div className="w-full h-full">
    <div className="flex">
      {React.Children.map(props.children, (tab, i) => <button className={"flex-1 text-center p-2 font-semibold text-sm" + (i == activeTab ? " bg-blue-500 text-white" : " bg-gray-100")} onClick={() => setActiveTab(i)}>{tab.props.name}</button>)}
    </div>
    <div className="h-full overflow-scroll">
      {displayChild}
    </div>
  </div>
}


function Tab(props) {
  return <div>
    {props.children}
  </div>
}


function DrawToolTray(props) {
  return <div className="w-72 border-l h-full">
    <TabGroup>
      <Tab name="Adjudicators"></Tab>
      <Tab name="Teams"></Tab>
    </TabGroup>
  </div>
}

function DrawEditor(props) {
  function onDragEnd(from, to, isSwap) {
    let changedDebates = simulateDragOutcome(draw, from, to, isSwap);

    executeAction("UpdateDraw", {
        updated_ballots: Object.keys(changedDebates).map(key => changedDebates[key].ballot)
    });
  }

  let currentView = {type: "Draw", uuid: props.round_uuid};
  let draw = useView(currentView, {"debates": []});
  console.log(draw.debates);
  let debates = draw.debates;

  let dragEnd = useCallback(makeDragHandler(onDragEnd), [draw]);

  return <div className="flex flex-row w-full h-full">
    <DndContext collisionDetection={closestCenter} onDragEnd={dragEnd}>
      <div className="flex-1 overflow-y-scroll">
        <table className="w-full">
          <tbody>
            {debates.map((debate) => <DebateRow key={debate.uuid} debate={debate} />)}
          </tbody>
        </table>
      </div>
      <DrawToolTray />
    </DndContext>
  </div>
  }


export default DrawEditor;
