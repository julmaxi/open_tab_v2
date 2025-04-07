import { useState, useId, useMemo, useEffect, useCallback } from "react";
import { Children } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { WebviewWindow, getCurrent } from '@tauri-apps/api/window'
import { emit, listen } from '@tauri-apps/api/event'

import ConnectivityStatus from "./ConnectivityStatus";

import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import { Outlet, Route, useParams } from "react-router";
import { Link } from "react-router-dom";
import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { useContext } from 'react';
import TournamentManager from "./Setup/TournamentOverview";
import LoginWindow from "./LoginWindow";
import UpdateProgressWindow from "./UpdateProgressWindow";
import { ErrorHandlingContext } from "./Action.js";


function NavGroup(props) {
  return <div className="ml-3">
    <h4 className="font-bold">{props.header}</h4>
    <ul>
      {Children.map(props.children, ((child, index) => <li key={index}>{child}</li>))}
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

  let clashesView = useView({type: "Clashes", tournament_uuid: tournamentContext.uuid}, {pending_clashes: [], approved_clashes: [], rejected_clashes: []});
  let pendingBallotsView = useView({type: "PendingBallots", tournament_id: tournamentContext.uuid}, {pending_ballot_counts: {}});

  console.log(pendingBallotsView);
  console.log(rounds);

  // Final item is a buffer so we never have the last item blocked by the connectivity status
  return <nav className="bg-gray-100 w-60 h-full overflow-y-scroll">
    <NavItem href="/">
      Assistant
    </NavItem>
    {
      rounds.map((round) => 
        <NavGroup header={round.name} key={round.uuid}>
          <NavItem href={`/round/${round.uuid}/draw`}>Draw</NavItem>
          <NavItem href={`/round/${round.uuid}/publish`}>Publish</NavItem>
          <NavItem href={`/round/${round.uuid}/results`}>
            Results
            {(pendingBallotsView.pending_ballot_counts[round.uuid] || []) > 0 ? <span className="ml-1 bg-red-500 text-white rounded-full px-2">{pendingBallotsView.pending_ballot_counts[round.uuid]}</span> : null}
          </NavItem>
        </NavGroup>
      )
    }
    <NavGroup header="Participants">
      <NavItem href="/participants">
        Participants
      </NavItem>
      <NavItem href="/clashes">
        Declarations {clashesView.pending_clashes.length > 0 ? <span className="bg-red-500 text-white rounded-full px-2">{clashesView.pending_clashes.length}</span> : null}
      </NavItem>
      <NavItem href="/institutions">
        Institutions
      </NavItem>
    </NavGroup>
    <NavItem href="/rounds">
      Rounds
    </NavItem>
    <NavItem href="/tab">
      Tab
    </NavItem>
    <NavGroup header="Feedback">
      <NavItem href="/feedback">
        Submissions
      </NavItem>
      <NavItem href="/feedback-config">
        Settings
      </NavItem>
      <NavItem href="/feedback-progress">
        Progress
      </NavItem>
    </NavGroup>
    <NavItem href="/venues">
      Venues
    </NavItem>
    <NavItem href="/status">
      Status
    </NavItem>
    <div className="h-8">
    </div>
  </nav>
}

function Main(props) {
  return <div className="flex-1 flex h-full overflow-hidden">
      {props.children}
  </div>
}

function WindowFrame(props) {
  return <div className="flex h-screen overscroll-none">
    <SideNav />
    <div className="absolute bottom-0 left-0">
        <ConnectivityStatus state="ok" lastUpdate="2 minutes" message="Connection is stable." />
    </div>

    <Main>
      <Outlet />
    </Main>
  </div>
}

function cleanErrors(errors) {
  let currTime = new Date().getTime();
  return errors.filter((error) => currTime - error.time < 5000);
}

export function TournamentWindow({tournamentId}) {
  let [currentErrors, setCurrentErrors] = useState([]);
  return <TournamentContext.Provider value={({uuid: tournamentId})}>
    <ErrorHandlingContext.Provider value={({handleError: (error) => {
      let currTime = new Date().getTime();
      setCurrentErrors([...currentErrors, {error, time: currTime}]);
      setTimeout(() => setCurrentErrors((errors) => {return cleanErrors(errors);}), 5000);
    }})}>
    <div className="overscroll-none">
      <div className="absolute z-20 top-0 left-0">
        <ul className="">
          {currentErrors.map((error, index) => <li className="bg-red-500 font-bold text-white shadow-sm rounded-md p-2 mb-1" key={index}>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-6 h-6 inline-block">
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />
            </svg>

            <span className="inline-block pl-1">{error.error}</span>
            </li>)}
        </ul>
      </div>
      <WindowFrame />
    </div>
    </ErrorHandlingContext.Provider>
  </TournamentContext.Provider>
}

export function App() {
  let label = getCurrent().label;

  let view = null;

  let [prefix, ...arg] = label.split(":");

  switch (prefix) {
    case "login":
      view = <LoginWindow />;
      break;
    case "update":
        view = <UpdateProgressWindow />;
        break;
    case "main":
      view = <TournamentManager />;
      break;
    default:
      view = <TournamentWindow tournamentId={arg[0]} />;
  }

  return view;
}

export default App;
