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
import { RoundPublicationRoute } from "./RoundPublicationView";
import { FeedbackOverviewRoute, FeedbackDetailViewRoute } from "./FeedbackView";

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
        path: "round/:roundId/publish",
        element: <RoundPublicationRoute />,
      },
      {
        path: "participants",
        element: <ParticipantOverview />
      },
      {
        index: true,
        path: "rounds",
        element: <RoundsEditorRoute />
      },
      {
        path: "feedback",
        element: <FeedbackOverviewRoute />,
        children: [
          {
            path: ":participantId",
            element: <FeedbackDetailViewRoute />
          }
        ]
      },
    ],
  },
]);



ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
