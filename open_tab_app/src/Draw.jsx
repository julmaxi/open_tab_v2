//@ts-check
import React, { useState, useId, useMemo, useEffect, useCallback, useContext } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'
import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin, DragOverlay} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DragItem, DropList, DropSlot, DropWell, makeDragHandler} from './DragDrop.jsx';

import { useView, updatePath, getPath, clone } from './View.js'
import { executeAction } from "./Action";
import { TournamentContext } from "./TournamentContext";
import { TabGroup, Tab } from "./TabGroup";
import { useSelect } from "downshift";

import { useSpring, useSpringRef, animated } from '@react-spring/web'


const TRAY_DRAG_PATH = "__tray__";

const ISSUE_COLORS_BG = {
  neutral: "bg-gray-100",
  none: "bg-green-500",
  misc: "bg-gray-500",
  low: "bg-blue-500",
  mid: "bg-yellow-500",
  high: "bg-red-500"
}

const ISSUE_COLORS_BORDER = {
  misc: "border-gray-500",
  low: "border-blue-500",
  mid: "border-yellow-500",
  high: "border-red-500"
}

let SWAP_BASE_COLORS = {
  neutral: null,
  none: [34, 197, 94],
  misc: [107, 114, 128],
  low: [34, 197, 94],
  mid: [234, 179, 8],
  high: [239, 68, 68]
}

let SWAP_ISSUE_GRADIENTS = Object.fromEntries(Object.entries(SWAP_BASE_COLORS).map(
  ([key, color]) => {
    if (color === null) {
      return [key, null];
    }
    let [r, g, b] = color;
    let r2 = Math.min(255, r + 20);
    let g2 = Math.min(255, g + 20);
    let b2 = Math.min(255, b + 20);

    return [key, `repeating-linear-gradient(45deg, rgb(${r} ${g} ${b}), rgb(${r} ${g} ${b}) 20px,rgb(${r2} ${g2} ${b2}) 20px, rgb(${r2} ${g2} ${b2}) 40px)`]
  }
));



function DragBox(props) {
  let highlightedIssues = props.highlightedIssues || [];
  let sortedIssues = highlightedIssues.sort((a, b) => b.severity - a.severity);
  let maxIssueSeverity = sortedIssues.length > 0 ? sortedIssues[0].severity : 0;
  let severityBucket = severityToBucket(maxIssueSeverity);
  let issueColor = ISSUE_COLORS_BORDER[severityBucket];

  let swapHighlightSeverity = props.swapHighlightSeverity;
  let swapIssueColor = null;
  if (swapHighlightSeverity !== null) {
    swapIssueColor = SWAP_ISSUE_GRADIENTS[swapHighlightSeverity]
  }

  const [animationProps, api] = useSpring(
    () => ({
      from: { opacity: 0 },
      to: { opacity: props.highlightedIssues?.length > 0 ? 1 : 0 },
    }),
    [props.highlightedIssues?.length]
  )

  return <div
    className={`relative flex bg-gray-100 min-w-[14rem] p-1 rounded`}
    style={{
      background: swapIssueColor
    }}
  >
    <div className="flex-1">
      {props.children}
    </div>
    <div className="flex items-center mr-1">
        <ClashIndicator issues={props.issues} onHover={props.onHighlightIssues} />
    </div>
    {props.highlightedIssues?.length > 0 ? <animated.div style={animationProps} className={`absolute w-full h-full top-0 left-0 border-4 rounded text-white ${issueColor}`}>
      <div className={`absolute top-0 right-0 text-xs p-0.5 rounded-bl ${ISSUE_COLORS_BG[severityBucket]}`}>
        <p>{props.highlightedIssues[0].type}</p>
        {props.highlightedIssues.length > 1 ? `+${props.highlightedIssues.length - 1} more`: []}
      </div>
    </animated.div>: []}
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

  //let highlightedIssues = all_participant_issues.filter((i) => props.issueHightlightedParticipantUuids.includes(i.target_participant_id));

  return <DragBox
    issues={props.team.issues}
    onHighlightIssues={(shouldHighlight) => {
      if (shouldHighlight) {
        //props.onHighlightIssues(props.team.members.map((m) => m.uuid));
        props.onHighlightIssues(props.team.uuid);
      }
      else {
        props.onHighlightIssues(null);
      }
    }}
    highlightedIssues={props.highlightedIssues}
  >
      <div>{props.team.name}</div>
      <HorizontalList>
        {props.team.members.map((member) => <div key={member.uuid} className="text-xs">{member.name}</div>)}
      </HorizontalList>
      <HorizontalList>
        {unique_participant_institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}


function severityToBucket(severity) {
  if (severity >= 75) {
    return "high";
  } else if (severity >= 50) {
    return "mid";
  } else if (severity >= 25) {
    return "low";
  } else {
    return "misc";
  }
}


function bucketIssuesBySeverity(issues) {
  let issueBuckets = issues.reduce((acc, issue) => {
    let bucket = severityToBucket(issue.severity);
    acc[bucket].push(issue);
    return acc;
  }, {misc: [], low: [], mid: [], high: []});
  return issueBuckets;
}


function ClashIndicator(props) {
  let issueBuckets = bucketIssuesBySeverity(props.issues);

  return <div className="font-mono font-bold flex h-6 rounded-md overflow-hidden w-16 border border-gray-600 text-xs" onMouseEnter={() => props.onHover(true)} onMouseLeave={() => props.onHover(false)}>
    {
      props.issues.length == 0 ?
      <div className="h-full flex-1 flex items-center justify-center bg-green-500 text-white pl-1 pr-1 text-lg">{"\u2713"}</div>
      :
      ["misc", "low", "mid", "high"].map(
        (key) => {
          return issueBuckets[key].length > 0 ?
          <div key={key} className={`h-full flex flex-1 items-center justify-center text-white pl-1 pr-1 ${ISSUE_COLORS_BG[key]}`}>{issueBuckets[key].length <= 9 ? issueBuckets[key].length : <span className="text-[8px]">&gt;9</span>}</div>
          :
          null
        }
      )
    }
  </div>;
}


function SpeakerItem(props) {
  return <DragBox issues={props.speaker.issues} onHighlightIssues={(shouldHighlight) => {
    if (shouldHighlight) {
      props.onHighlightIssues(props.speaker.uuid);
    }
    else {
      props.onHighlightIssues(null);
    }
  }} highlightedIssues={props.highlightedIssues}>
    <div>{props.speaker.name}</div>
    <div className="text-xs">{props.speaker.team_name}</div>
    <HorizontalList>
      {props.speaker.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}


function AdjudicatorItem(props) {
  //let highlightedIssues = props.adjudicator.issues.filter((i) => props.issueHightlightedParticipantUuids.includes(i.target_participant_id));
  let highlightedIssues = props.highlightedIssues;
  let swapIssueSeverity = null;

  if (props.dragSwapHighlight !== null) {
    highlightedIssues = [];

    swapIssueSeverity = props.dragSwapHighlight.severityBucket;
  }
  return <DragBox issues={props.adjudicator.issues} swapHighlightSeverity={swapIssueSeverity} onHighlightIssues={(shouldHighlight) => {
    if (shouldHighlight) {
      props.onHighlightIssues(props.adjudicator.uuid);
    }
    else {
      props.onHighlightIssues(null);
    }
  }} highlightedIssues={highlightedIssues}>
    <div className={props.adjudicator.is_available ? "": "line-through"}>{props.adjudicator.name}</div>
    <HorizontalList>
      {props.adjudicator.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
    </HorizontalList>
  </DragBox>
}

function find_issues_with_target(ballot, target_uuid) {
  return {
    "government": ballot.government !== null ? filter_issues_by_target(ballot.government.issues, target_uuid) : [],
    "opposition": ballot.opposition !== null ? filter_issues_by_target(ballot.opposition.issues, target_uuid) : [],
    "adjudicators": ballot.adjudicators !== null ? ballot.adjudicators.map(adj => filter_issues_by_target(adj.issues, target_uuid)) : [],
    "non_aligned_speakers": ballot.non_aligned_speakers !== null ? ballot.non_aligned_speakers.map(speaker => filter_issues_by_target(speaker.issues, target_uuid)) : []
  }
}

function filter_issues_by_target(issues, target_uuid) {
  return issues.filter((i) => i.target.uuid === target_uuid);
}


function VenueSelector(props) {
  let tournamentId = useContext(TournamentContext).uuid;

  let venues = useView({type: "Venues", tournament_uuid: tournamentId}, {venues: []});

  let selectedItem = props.venue ? venues.venues.find((v) => v.uuid === props.venue.uuid) : null;

  const {
    isOpen,
    getToggleButtonProps,
    getMenuProps,
    highlightedIndex,
    closeMenu,
    getItemProps,
  } = useSelect({
    items: venues.venues,
    itemToString: item => (item ? item.name : ""),
    selectedItem: selectedItem || null,
  });
  const springRef = useSpringRef()

  let style = useSpring({
    from: { height: 0},
    to: {
      height: isOpen ? 240: 0
    },
  });

  return <div className="inline">
    <button type="button" {...getToggleButtonProps()}>
      {selectedItem ? selectedItem.name : "<No Venue>"}
    </button>
    <div className="w-0 h-0 relative z-40">
      <animated.div className="w-72 bg-white mt-1 shadow-md overflow-auto p-0 h-8" style={style}>
      <ul {...getMenuProps()} className="w-full" >
        {isOpen &&

          venues.venues.map((item, index) => (
            <li key={item.name} {...getItemProps({ item, index })} onClick={() => {
              props.onVenueChange(item);
              closeMenu();
            }}>
              {item.name}
            </li>
          ))
        }
      </ul>
      </animated.div>
    </div>
  </div>
}


function DebateRow(props) {
  let ballot = props.debate.ballot;
  let [localHighlightedIssues, setLocalHighlightedIssues] = useState({
    "government": [],
    "opposition": [],
    "adjudicators": [],
    "non_aligned_speakers": []
  });

  let highlightedIssues = props.dragHighlightedIssues ? props.dragHighlightedIssues : localHighlightedIssues;
  
  return <>
    <tr>
      <td colSpan={4}>
        Debate {props.debate.index + 1}: <VenueSelector venue={props.debate.venue} onVenueChange={(venue) => props.onVenueChange(venue)} />
      </td>
    </tr>
    <tr>
      <td className="border">
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "government"]}>
          {ballot.government !== null ? <TeamItem
            team={ballot.government}
            onHighlightIssues={
              (uuid) => setLocalHighlightedIssues(find_issues_with_target(ballot, uuid))
            }
            highlightedIssues={
              highlightedIssues.government
            }
            /> : []}
        </DropWell>
        <br />
        <DropWell type="team" collection={["debates", props.debate.index, "ballot", "opposition"]}>
          {ballot.opposition !== null ? <TeamItem
            team={ballot.opposition}
            onHighlightIssues={
              (uuid) => setLocalHighlightedIssues(find_issues_with_target(ballot, uuid))
            }
            highlightedIssues={
              highlightedIssues.opposition
            }
            /> : []}
        </DropWell>
      </td>
      <td className="border">
        <DropList type="speaker" collection={["debates", props.debate.index, "ballot", "non_aligned_speakers"]}>
          {ballot.non_aligned_speakers.map((speaker, idx) =>
            <SpeakerItem
            key={speaker.uuid}
            speaker={speaker}
            onHighlightIssues={
              (uuid) => setLocalHighlightedIssues(find_issues_with_target(ballot, uuid))
            }
            highlightedIssues={
              highlightedIssues.non_aligned_speakers[idx]
            }
            />)}
        </DropList>
      </td>
      <td className="border">
        <DropList minWidth={"200px"} type="adjudicator" collection={["debates", props.debate.index, "ballot", "adjudicators"]}>
          {ballot.adjudicators.map((adjudicator, idx) =>
            <AdjudicatorItem
            key={adjudicator.uuid}
            adjudicator={adjudicator}
            onHighlightIssues={
              (uuid) => {
                setLocalHighlightedIssues(find_issues_with_target(ballot, uuid));
              }
            }
            highlightedIssues={
              highlightedIssues.adjudicators[idx]
            }
            dragSwapHighlight={props.dragSwapHighlight && props.dragSwapHighlight.adjudicatorId == adjudicator.uuid ? props.dragSwapHighlight : null}
          />)}
        </DropList>
      </td>
      <td className="border">
        <DropWell
          minWidth={"200px"}
          type="adjudicator"
          collection={["debates", props.debate.index, "ballot", "president"]}
        >
          {ballot.president ? <AdjudicatorItem adjudicator={ballot.president} onHighlightIssues={() => {}} dragSwapHighlight={props.dragSwapHighlight && props.dragSwapHighlight.adjudicatorId == ballot.president.uuid ? props.dragSwapHighlight : null} /> : []}
        </DropWell>
      </td>
    </tr>
  </>;
}

function simulateDragOutcome(draw, from, to, isSwap) {
  if (from.collection === TRAY_DRAG_PATH) {
    if (to.collection == TRAY_DRAG_PATH) {
      return {}
    }

    let val = draw.adjudicator_index.find(
      (adjudicator) => adjudicator.adjudicator.uuid == from.index
    );

    if (val === undefined) {
      console.warn(`Could not find ${from.index}`);
      console.info(
        draw.adjudicator_index
      );
      return {};
    }

    if (val.position.type === "NotSet") {
      let to_collection = clone(getPath(draw, to.collection));
      let to_debate = clone(draw.debates[to.collection[1]]);
      
      if (to.index !== undefined) {
        if (isSwap) {
          to_collection[to.index] = val.adjudicator;
        }
        else {
          to_collection.splice(to.index, 0, val.adjudicator);
        }
      }
      else {
        to_collection = val.adjudicator;
      }
  
      updatePath(to_debate, to.collection.slice(2), to_collection);
    
      return {[to.collection[1]]: to_debate};  
    }
    else {
      if (val.position.position.type === "Panel") {
        from.collection = ["debates", val.position.debate_index, "ballot", "adjudicators"];
        from.index = val.position.position.position;
      }
      else if (val.position.position.type === "President") {
        from.collection = ["debates", val.position.debate_index, "ballot", "president"];
        from.index = undefined;
      }
    }
  }

  var from_debate = clone(draw.debates[from.collection[1]]);
  var from_collection = clone(getPath(draw, from.collection));

  var to_debate;
  if (from.collection[1] == to.collection[1]) {
    to_debate = from_debate;
  }
  else {
    to_debate = clone(draw.debates[to.collection[1]]);
  }

  if (to.collection === TRAY_DRAG_PATH) {
    if (from.index !== undefined) {
        from_collection.splice(from.index, 1);
    }
    else {
      from_collection = null;
    }
    updatePath(from_debate, from.collection.slice(2), from_collection);
    return {[from.collection[1]]: from_debate};
  }
  else {
    var to_collection;
    if (from.collection == to.collection) {
      to_collection = from_collection
    }
    else {
      to_collection = clone(getPath(draw, to.collection));
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
  }

  updatePath(from_debate, from.collection.slice(2), from_collection);
  updatePath(to_debate, to.collection.slice(2), to_collection);

  if (from.collection[1] == to.collection[1]) {
    return {[from.collection[1]]: from_debate};
  }
  else {
    return {[from.collection[1]]: from_debate, [to.collection[1]]: to_debate};
  }
}


function adjPositionToStr(position) {
  if (position.type == "NotSet") {
    return "-"
  }
  else {
    let chairStr = "";
    
    if (position.position.type == "President") {
      chairStr = "Pres.";
    }
    else {
      let isChair = position.position.position == 0;
      chairStr = isChair ? "Chair" : "Panel";
    }

    return `${chairStr} ${position.debate_index + 1}`

  }
}

function teamPositionToStr(position) {
  if (position.type == "NotSet") {
    return "-"
  }
  else if (position.type == "NonAligned") {
    let positions = Object.entries(position.member_positions).map(
      ([_, p]) => p.debate_index + 1
    )

    return `Non. ${positions.join(", ")}`
  }
  else {
    const abbreviations = {
      "Government": "Gov.",
      "Opposition": "Opp.",
    }
    return `${abbreviations[position.role] || "<Unknown>"} ${position.debate_index + 1}`

  }
}

function AdjudicatorTable({adjudicator_index, ...props}) {
  return <div className="h-full overflow-auto">
    <table className="w-full text-sm">
      <thead className="sticky top-0 bg-white">
        <tr>
          <th>Name</th>
          <th>Position</th>
        </tr>
      </thead>
      <tbody className="w-full">
      {
        adjudicator_index.map(
          (adj, idx) => {
            return <DragItem content_tag="tr" key={idx} collection={TRAY_DRAG_PATH} index={adj.adjudicator.uuid} type={"adjudicator"}>
              <td className={
                adj.is_available ? "" : "line-through"
              }>{adj.adjudicator.name}</td>
              <td>{adjPositionToStr(adj.position)}</td>
            </DragItem>
          }
        )
      }
      </tbody>
    </table>
  </div>
}

function TeamTable({team_index, ...props}) {
  return <div className="h-full overflow-auto">
    <table className="w-full text-sm">
      <thead className="sticky top-0 bg-white">
        <tr>
          <th>Name</th>
          <th>Position</th>
        </tr>
      </thead>
      <tbody className="w-full">
        {
          team_index.map(
            (entry, idx) => <TeamIndexEntry key={entry.team.uuid} entry={entry} />
          )
        }
      </tbody>
    </table>
  </div>
}


function SpeakerIndexEntries({team, positions, ...props}) {
  let rows =  team.members.map(
    (member) => {
      let position = positions[member.uuid];
      return <tr>
        <td className="pl-4">{member.name}</td>
        <td>{position.debate_index + 1}{position.position !== undefined ? ` (${position.position + 1})` : [] }</td>
      </tr>;
    }
  );
  return <>
    {rows}
  </>
}


function TeamIndexEntry({entry, ...props}) {
  let [isExpanded, setIsExpanded] = useState(false);

  return <>
    <tr onClick={() => {
      setIsExpanded(!isExpanded);
    }}>
      <td>{entry.team.name}</td>
      <td>{teamPositionToStr(entry.position)}</td>
    </tr>
    {isExpanded ? <SpeakerIndexEntries team={entry.team} positions={entry.position.member_positions || Object.fromEntries(
      entry.team.members.map(
        (member) => [member.uuid, entry.position]
      )
    )} /> : []}

  </>
}

const DropIndicator = ({ visible }) => {
  return (
    <div 
      className={`absolute w-full h-full bg-black bg-opacity-50 z-10 flex items-center justify-center 
      ${visible ? '' : 'hidden'}`}>
      <div className="text-center py-3 px-6 rounded-lg">
        <svg className="w-12 h-12 mx-auto mb-2 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
        <p className="text-white">Remove</p>
      </div>
    </div>
  )
}


function DrawToolTray({adjudicator_index, team_index, isDragging, ...props}) {
  return <div className="w-72 border-l h-full relative">
    <DropSlot collection={TRAY_DRAG_PATH} type={"adjudicator"} className={"h-full"}>
      <DropIndicator visible={isDragging} />
      <TabGroup>
        <Tab name="Adjudicators" autoScroll={false}>
          <AdjudicatorTable adjudicator_index={adjudicator_index} />
        </Tab>
        <Tab name="Teams">
          <TeamTable team_index={team_index} />
        </Tab>
      </TabGroup>
    </DropSlot>
  </div>
}

function getDragInfoFromDragInfo(drag_info, draw) {
  if (drag_info.collection === TRAY_DRAG_PATH) {
    if (drag_info.type == "adjudicator") {
      return draw.adjudicator_index.find(
        (adj) => adj.adjudicator.uuid == drag_info.index
      ).adjudicator;
    }
    else if (drag_info.type == "team") {
      return draw.team_index.find(
        (team) => team.team.uuid == drag_info.index
      ).team;
    }
    else if (drag_info.type == "speaker") {
      for (let team of draw.team_index) {
        let member = team.members.find(
          (member) => member.uuid == drag_info.index
        );
        if (member !== undefined) {
          return member;
        }
      }
    }
  }
  else {
    let collection = getPath(draw, drag_info.collection);
    if (drag_info.index !== undefined) {
      return collection[drag_info.index];
    }
    else {
      return collection;
    }
  }
}


function DragItemPreview({item, highlight, ...props}) {
  let issueColor = highlight ? ISSUE_COLORS_BG[highlight] : "bg-gray-100";

  return <div className={`${issueColor} min-w-[14rem] p-1 rounded`}>
    {item.name}
  </div>
}

function getMaxSeverityFromEvaluationResult(result) {
  let allIssues = [];
  for (let issues of Object.values(result)) {
    for (let elem of issues) {
      if (Array.isArray(elem)) {
        allIssues.push(...elem);
      }
      else {
        allIssues.push(elem);
      }
    }
  }
  let maxSeverity = Math.max(0, ...allIssues.map((issue) => issue.severity));
  return maxSeverity;

}


function DrawEditor(props) {
  function onDragEnd(from, to, isSwap) {
    setDragHighlightedIssues(null);
    setDraggedItemHighlight(null);
    setDraggedItem(null);
    setDragSwapHighlight({
      severityBucket: null,
      debateIdx: null,
      adjudicatorId: null
    });
    let changedDebates = simulateDragOutcome(draw, from, to, isSwap);

    executeAction("UpdateDraw", {
        updated_ballots: Object.keys(changedDebates).map(key => changedDebates[key].ballot)
    });
  }

  function onDragOverFunc(from, to, isSwap) {
    if (dragHighlightedIssues === null) {
      return;
    }
    if (to.collection === TRAY_DRAG_PATH) {
      setDraggedItemHighlight("neutral");
      return;
    }

    if (to.collection !== TRAY_DRAG_PATH) {
      let draggedAdjudicatorId = null;

      if (from.collection == TRAY_DRAG_PATH) {
        draggedAdjudicatorId = from.index;
      }
      else {
        if (from.index !== undefined) {
          draggedAdjudicatorId = getPath(draw, from.collection)[from.index].uuid;
        }
        else {
          draggedAdjudicatorId = getPath(draw, from.collection).uuid;
        }
      }

      let outcome = simulateDragOutcome(draw, from, to, isSwap);
      let dragTargetRoomId = to.collection[to.collection.length - 3];
      let targetRoom = outcome[dragTargetRoomId].ballot;
  
      invoke("evaluate_ballots", {tournamentId: tournament.uuid, roundId: roundId, ballots: [targetRoom], targetUuid: draggedAdjudicatorId}).then(
        (issues) => {
          let maxSeverity = getMaxSeverityFromEvaluationResult(issues[0]);
          let severityBucket = maxSeverity == 0 ? "none" : severityToBucket(maxSeverity);
          setDraggedItemHighlight(severityBucket);
        }
      );
      
      if (isSwap) {
        let swapAdjudicatorId = null;
        if (to.index !== undefined) {
          swapAdjudicatorId = getPath(draw, to.collection)[to.index].uuid;
        }
        else {
          let collectionValue = getPath(draw, to.collection);

          if (collectionValue) {
            swapAdjudicatorId = collectionValue.uuid;
          }
        }
        if (swapAdjudicatorId !== null && from.collection !== TRAY_DRAG_PATH) {
          let dragSourceRoomId = from.collection[from.collection.length - 3];
          let sourceRoom = outcome[dragSourceRoomId].ballot;
          invoke("evaluate_ballots", {tournamentId: tournament.uuid, roundId: roundId, ballots: [sourceRoom], targetUuid: swapAdjudicatorId}).then(
            (issues) => {
              let maxSeverity = getMaxSeverityFromEvaluationResult(issues[0]);
              let severityBucket = maxSeverity == 0 ? "none" : severityToBucket(maxSeverity);
              if (draggedItem !== null) {
                setDragSwapHighlight({
                  severityBucket: severityBucket,
                  debateIdx: dragTargetRoomId,
                  adjudicatorId: swapAdjudicatorId
                });
              }
            }
          );
        }
      }
      else {
        setDragSwapHighlight({
          severityBucket: null,
          debateIdx: null,
          adjudicatorId: null
        });
      }
    }

    /*
    if (to.collection[to.collection.length - 1] !== "president") {
      let roomIndex = to.collection[1];
      let targetRoomIssues = dragHighlightedIssues[roomIndex];
      
      let allIssues = [];
      for (let issues of Object.values(targetRoomIssues)) {
        for (let elem of issues) {
          if (Array.isArray(elem)) {
            allIssues.push(...elem);
          }
          else {
            allIssues.push(elem);
          }
        }
      }
      let maxSeverity = Math.max(0, ...allIssues.map((issue) => issue.severity));

      let severityBucket = maxSeverity == 0 ? "none" : severityToBucket(maxSeverity);

      setDraggedItemHighlight(severityBucket);
    }
    else {
      setDraggedItemHighlight("neutral");
    }
    */
  }
  const onDragOver = makeDragHandler(onDragOverFunc);

  let currentView = {type: "Draw", uuid: props.round_uuid};
  let draw = useView(currentView, {"debates": [], "adjudicator_index": []});
  let debates = draw.debates;

  let roundId = props.round_uuid;

  let tournament = useContext(TournamentContext);

  let [dragHighlightedIssues, setDragHighlightedIssues] = useState(null);
  let [dragSwapHighlight, setDragSwapHighlight] = useState({
    severityBucket: null,
    debateIdx: null,
    adjudicatorId: null
  });
  let [draggedItem, setDraggedItem] = useState(null);
  let [draggedItemHighlight, setDraggedItemHighlight] = useState(null);


  let dragEnd = useCallback(makeDragHandler(onDragEnd), [draw]);
  let dragOver = useCallback(onDragOver, [draw, dragHighlightedIssues]);
  let dragStart = useCallback((x) => {
    setDraggedItem(x.active.data.current);
    if (x.active.data.current.type != "adjudicator") {
      return;
    }

    let simulatedBallots = [];

    for (let i = 0; i < debates.length; i++) {
      let outcome = simulateDragOutcome(
        draw,
        x.active.data.current,
        {
          index: 0,
          collection: ["debates", i, "ballot", "adjudicators"] 
        },
        false
      );
      simulatedBallots.push(outcome[i].ballot);
    }
    invoke("evaluate_ballots", {tournamentId: tournament.uuid, roundId: roundId, ballots: simulatedBallots, targetUuid: simulatedBallots[0].adjudicators[0].uuid}).then(
      (issues) => {
        setDragHighlightedIssues(issues);
      }
    );
  }, [draw, roundId]);

  let dragItemInfo = null;
  if (draggedItem) {
    dragItemInfo = getDragInfoFromDragInfo(draggedItem, draw);
  }

  return <div className="flex flex-row w-full h-full">
    <DndContext collisionDetection={closestCenter} onDragEnd={dragEnd} onDragOver={dragOver} onDragStart={dragStart}>
      
      <div className="flex-1 overflow-y-scroll">
        <table className="w-full">
          <tbody>
            {debates.map((debate, debateIdx) => <DebateRow
              key={debate.uuid}
              debate={debate}
              dragHighlightedIssues={dragHighlightedIssues ? dragHighlightedIssues[debateIdx] : null}
              dragSwapHighlight={dragSwapHighlight.debateIdx == debateIdx ? dragSwapHighlight : null}
              onVenueChange={(venue) => {
                executeAction("UpdateDraw", {updated_debates: [{...debate, venue: venue}]});
              }
            }
            />)}
          </tbody>
        </table>
      </div>

      <DrawToolTray adjudicator_index={draw.adjudicator_index} team_index={draw.team_index} isDragging={dragItemInfo !== null} />

      <DragOverlay dropAnimation={null}>
        {dragItemInfo ? <DragItemPreview item={dragItemInfo} highlight={draggedItemHighlight} /> : []}
      </DragOverlay>
    </DndContext>
  </div>
}


export default DrawEditor;
