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
  let currentView = {type: "RoundsOverview", tournament_uuid: "00000000-0000-0000-0000-000000000001"};

  let [rounds, setRounds] = useState([]);

  useEffect(() => {
    invoke("subscribe_to_view", {view: currentView}).then((msg) => {
      let rounds = JSON.parse(msg["success"]);
      console.log(rounds.rounds);
      setRounds(rounds.rounds);
    });
  }, []);

  useEffect(
    () => {
        const unlisten = listen('views-changed', (event) => {
          console.log("Reload")
        })

        return () => {
            unlisten.then((unlisten) => unlisten())
        }
    },
  [rounds]
  );

  return <nav className="bg-gray-100 w-60 h-full overflow-y-scroll">
    <NavGroup header="Rounds">
      {rounds.map((round) => <NavItem href={`/round/${round.uuid}/draw`} key={round.uuid}>{round.name}</NavItem>)}      
    </NavGroup>
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
  return <div className="overscroll-none">
    <WindowFrame />
  </div>
}

export function DrawEditorRoute() {
  let { roundId } = useParams();
  return <DrawEditor round_uuid={roundId} />;
}

export default App;
