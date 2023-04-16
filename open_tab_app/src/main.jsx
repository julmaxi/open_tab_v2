import React from "react";
import ReactDOM from "react-dom/client";
import {App, DrawEditorRoute} from "./App";
import "./styles.css";
import {
  createMemoryRouter,
  RouterProvider,
} from "react-router-dom";
import { ParticipantOverview } from "./ParticipantOverview";

import { RoundResultRoute } from "./Results";
import { RoundsEditorRoute } from "./RoundsEditor";

const router = createMemoryRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        path: "round/:roundId/draw",
        element: <DrawEditorRoute />,
      },
      {
        path: "round/:roundId/results",
        element: <RoundResultRoute />,
      },
      {
        path: "participants",
        element: <ParticipantOverview />
      },
      {
        index: true,
        path: "rounds",
        element: <RoundsEditorRoute />
      }
    ],
  },
]);



ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
