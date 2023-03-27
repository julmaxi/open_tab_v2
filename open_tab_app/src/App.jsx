import { useState, useId, useMemo, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'

import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DropList, DropWell, makeDragHandler} from './DragDrop.jsx';
import DrawEditor from "./Draw";
import { Outlet, Route, useParams } from "react-router";
import { Link } from "react-router-dom";
import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { useContext } from 'react';
import { ParticipantOverview } from "./ParticipantOverview";

function NavGroup(props) {
  return <div className="ml-3">
    <h4 className="font-bold">{props.header}</h4>
    <ul>
      {props.children.map((child, index) => <li key={index}>{child}</li>)}
    </ul>
  </div>
}

function NavItem(props) {
  return <div className="ml-3">
    <Link to={props.href}>{props.children}</Link>
  </div>
}

function SideNav(props) {
  let tournamentContext = useContext(TournamentContext);
  let currentView = {type: "RoundsOverview", tournament_uuid: tournamentContext.uuid};

  let roundsOverview = useView(currentView, {"rounds": []});
  let rounds = roundsOverview.rounds;

  return <nav className="bg-gray-100 w-60 h-full overflow-y-scroll">
    <NavGroup header="Rounds">
      {rounds.map((round) => <NavItem href={`/round/${round.uuid}/draw`} key={round.uuid}>{round.name}</NavItem>)}      
    </NavGroup>
    <NavItem href="/">
      Participants
    </NavItem>
  </nav>
}

function Main(props) {
  return <div className="flex-1 flex h-full overflow-hidden">
    <div className="flex-1 overflow-y-scroll">
      {props.children}
    </div>
  </div>
}

function WindowFrame(props) {
  return <div className="flex h-screen overscroll-none">
    <SideNav />
    <Main>
      <Outlet />
    </Main>
  </div>
}

export function App() {
  return <TournamentContext.Provider value={({uuid: "00000000-0000-0000-0000-000000000001"})}>
    <div className="overscroll-none">
      <WindowFrame />
    </div>
  </TournamentContext.Provider>
}

export function DrawEditorRoute() {
  let { roundId } = useParams();
  return <DrawEditor round_uuid={roundId} />;
}


export default App;
