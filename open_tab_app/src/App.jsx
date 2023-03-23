import { useState, useId, useMemo, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from '@tauri-apps/api/event'

import "./App.css";

import {DndContext, useDraggable, useDroppable, closestCenter, closestCorners, pointerWithin} from '@dnd-kit/core';
import {CSS} from '@dnd-kit/utilities';

import {DropList, DropWell, makeDragHandler} from './DragDrop.jsx';
import DrawEditor from "./Draw";


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
    {props.children}
  </div>
}

function SideNav(props) {
  return <nav className="bg-gray-100 w-60 h-full overflow-y-scroll">
    <NavGroup header="Rounds">
      <NavItem>Runde 1</NavItem>
      <NavItem>Runde 2</NavItem>
      <NavItem>Runde 3</NavItem>
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
      {props.children}
    </Main>
  </div>
}

function App() {
  return <div className="overscroll-none">
    <WindowFrame>
      <DrawEditor />
    </WindowFrame>
  </div>
}

export default App;
