import React from "react";
import ReactDOM from "react-dom/client";
import {App, DrawEditorRoute} from "./App";
import "./styles.css";
import {
  createMemoryRouter,
  RouterProvider,
} from "react-router-dom";
import { ParticipantOverview } from "./ParticipantOverview";


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
        index: true,
        element: <ParticipantOverview />
      }
    ],
  },
]);



ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
