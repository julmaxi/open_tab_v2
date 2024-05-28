import React from "react";
import ReactDOM from "react-dom/client";
import {App, DrawEditorRoute} from "./App";
import "./styles.css";
import {
  createMemoryRouter,
  RouterProvider,
} from "react-router-dom";
import ParticipantRoute from "./Routes/Participants";

import { RoundResultRoute } from "./Results";
import { RoundsEditorRoute } from "./RoundsEditor";
import { RoundPublicationRoute } from "./RoundPublicationView";
import { FeedbackOverviewRoute } from "./FeedbackView";
import { TabViewRoute } from "./TabView";
import { FeedbackConfigRoute } from "./FeedbackConfig";
import VenueViewRoute from "./Venues";
import TournamentViewRoute from "./TournamentView";
import { AssistantRoute } from "./Assistant";
import { FeedbackProgressRoute } from "./FeedbackProgress";

const router = createMemoryRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        path: "/",
        element: <AssistantRoute />
      },
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
        element: <ParticipantRoute />
      },
      {
        index: true,
        path: "rounds",
        element: <RoundsEditorRoute />
      },
      {
        path: "feedback",
        element: <FeedbackOverviewRoute />,
      },
      {
        path: "feedback-config",
        element: <FeedbackConfigRoute />,
      },
      {
        path: "feedback-progress",
        element: <FeedbackProgressRoute />,
      },
      {
        path: "tab",
        element: <TabViewRoute />,
      },
      {
        path: "venues",
        element: <VenueViewRoute />,
      },
      {
        path: "status",
        element: <TournamentViewRoute />,
      },
    ],
  },
]);



ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
