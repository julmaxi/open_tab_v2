import React, { useEffect, useState } from 'react';
import { useContext } from 'react';
import { TournamentContext } from './TournamentContext';
import {listen} from '@tauri-apps/api/event';
import { useView } from './View';
import ModalOverlay from './UI/Modal';
import LoginWidget from './LoginWidget';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from "./settings";


const UPDATE_TIMEOUT_MILLIS = 30000;

/**
 * 
 * @param {Object} props
 * @param {string} props.state - The current connectivity state, e.g., 'ok', 'warning', 'no_connection', 'error', 'require_password'.
 * @param {Date} props.lastUpdate - The last time the connectivity status was updated.
 * @param {string} props.message - A message to display in the drawer, if any.
 * @returns 
 */
function ConnectivityStatusDrawer({ state, lastUpdate, message, showDelayWarning }) {
    const getColor = (state) => {
        switch (state) {
            case 'ok': return 'rgb(34 197 94)';
            case 'warning': return 'rgb(234 179 8)';
            case 'no_connection': return 'rgb(107 114 128)';
            case 'error': return 'rgb(239 68 68)';
            case 'require_password': return 'rgb(239 68 68)';
            default: return 'rgb(107 114 128)';
        }
    };

    return (
        <div className="rounded-tr-lg bg-white p-2 shadow-lg h-8 w-44 transition-all flex justify-end relative right-36 hover:right-0">
                <p className='text-xs pr-2'>
                    {
                        lastUpdate ? <>Last update {lastUpdate.toLocaleTimeString(
                            {
                                hour: '2-digit',
                            }
                        ) }</> : "No update this session"
                    }
                    </p>
                <svg className='' height="100%" viewBox="0 0 54 54" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlSpace="preserve">
                    <defs>
                        <image id="_Image1" width="54px" height="54px" xlinkHref="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADYAAAA2CAYAAACMRWrdAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAE+0lEQVRogc2a6XIjNwyEvzl0RXKS3fd/x9RuZW3rzg+yBQwlawdzOO4q1lgekcMmwAbAUQXUubVAg+Hq2qX429//khCZNbABVvmzJn0mETkDp+J65p70l4GI/QF8y9dHOANH1/bAwX3WAnwZkt5iH5Eif0ffE/bAe257uiRF9H8jqcm+0J10H7S5z460KMvctAjVdNOMo83XsZNY5AbJaq+5vWHWlKt+ChqSYOzydaoxZckFSXHLhZvdPecgJlR5zC3JM0TwU/bcnMSEChOncu/NRlLEtsTFI4qaFCvXdK03i3I2pH3wO7mfEi1pIZUI+Pg3GZr8oJYk1e3zr0+KVW6eHExE0Pu8UiQNXDF/LGpJ7lmmZqPJVSR/l0susABbu3ttvrciWXYO/AP8IMU/xb3BBFss2b3mAet8r3LNk1+S9qTaVPiGLeovUi6qeYXxyDLePeUiyuoPWI74ltsJK3vGYkVaPD1zsFv65HaH5X1rbHPLPUX2QiKjLF+J8BFbnDHQltBzBrmkFFFly7a4r8EPmLVUrqgu27t7r6TFecFyxyHY5Pl4q4XyTAXov3IrUeXvLPPDVAUoNMiSpRX3+X9LhivrIvfVAobcckjZ4suVdf7sQ4YIysoSnSFQnFOdFyI2JleUG5dpkid4zN8dqqAbukcSvchNlQSL4AoLF5qMCF7yJIe45tqN1UtMps7uF1iZAibbEpsjdmAUQZ3H9qL1lNxcZcuKtMpSs1PRFsTDgvayyD1VyTnLFp1+wT25C8PIregeGn1otbnLliqPq5Dg9901Pzfqli0WSz8MAZ9Vtkg1veVUqmyJCUqb++2xY747PCtb6kcdRmCVn+WlWwoX9ZYFZrWHKik3kHK9k9KiX/m6z5OoiLvMI0igJACyXE1sj9ckMgfMrTvQZEv1UlqkBPctX6+MS5MgEfDPkuX82WQfLDGr3QnJo7LFZxAXLD3au3ZlXHhQwC2lO7LfNMeHVntUtmzplisyu1ZZ2fwBC5xRVKQVL6W7fD/wOyzdfDpWU+a+A75jpYsI+kzeW1GljCR3SAyUcHkRqEhpV9/9rDAiLbgp5LOyRdbYkEjquBq6ZYpWfE187yngendq8jP7oiFpwIGCWJ+yRcfVO7qC4wPuCYtXEdR0X1yAlUN90GAacHPHaK6o42otgiwngsrgI+QWdIWE3D8S284k1b6549AkWHmg33N64UdwUhrPW00L2NdqNSnu3txxTHavPBAswCsmaWJ90WKCJKtpf/dBg71ZPQPXKcqWDfcB90o84NbYadeVRPYl0F/ueASuU+WD3/MkliSSb8DP4Bg68mswCX8P9r+9aJyKWAX8jcWgE8nnh5DT6bRy177Q8XsD02bwS5LV5NJ7UjIdOezUqqsKiBDTHBomtJjwJ2Y1ueS/gf7KdGoSMaVufaEsqZ6aWEUSoiWWoL4Gx1jRdcd9oO/tOH5qYmA5ZkP3CLwvbvsECyN9oXOUWYipaNSr2KgI+KRbYaQvbu/25iAGXRFQ4O0L/05A+WgEk8p9CdVzWvWIO6nw9WVSRFln22MavHSniEuVxCJWq3DE5vghSXkCNoTYkL6VBohu0L64uKtiUqSvt1ak7xksEOo0airo5bgmeHT/m7Ovfm13bknEFETLF3lReHnXi3eJh9IrBeBHv4gb0/fk+/qfOvjfd4yBP6PXquvAVcd7Hz1jsr7/AaUi/Vqd70R4AAAAAElFTkSuQmCC"/>
                    </defs>
                    <g transform="matrix(1,0,0,1,-123,-73)">
                        <g transform="matrix(1,-0,-0,1,123,73)">
                            <use xlinkHref="#_Image1" x="0" y="0" width="54px" height="54px" />
                        </g>
                        <path d="M125,105C136.038,105 145,113.962 145,125L125,125L125,105ZM125,90C144.317,90 160,105.683 160,125L150,125C150,111.202 138.798,100 125,100L125,90ZM125.323,75.001C152.663,75.174 174.827,97.338 174.999,124.677L175,125L165,125C165,102.923 147.077,85 125,85L125,75L125.323,75.001Z" style={{ "fill": getColor(state == "ok" ? (showDelayWarning ? "warning" : "ok") : state) }} />
                    </g>
                </svg>
        </div>
    );
}

function ConnectivityStatus() {
    let tournamentId = useContext(TournamentContext).uuid;

    let tournamentView = useView({type: "TournamentStatus", tournament_uuid: tournamentId}, null);

    let hasRemote = tournamentView == null ? false : tournamentView.remote_url != null;

    let [state, setState] = useState("no_connection");
    let [timestamp, setTimestamp] = useState(null);
    let [msg, setMsg] = useState("No connection.");
    let [showLogin, setShowLogin] = useState(false);
    let [defaultUserId, setDefaultUserId] = useState("");
    let [loginError, setLoginError] = useState(null);
    let timestampRef = React.useRef(timestamp);
    let [ showDelayWarning, setShowDelayWarning ] = useState(false)
    
    function updateWithStatus(status) {
        if (status.timestamp) {
            let parsedTimestamp = new Date(status.timestamp + "Z");
            timestampRef.current = parsedTimestamp
            setTimestamp(
                parsedTimestamp
            );
        }
        switch (status.status) {
            case 'Alive': 
                setState("ok");
                setShowLogin(false);
                break;
            case 'Error':
                setState("error");
                setShowLogin(false);
                break;
            case 'Connect':
                setState("ok");
                setShowLogin(false);
                break;
            case 'Disconnect':
                setState("no_connection");
                setShowLogin(false);
                break;
            case 'PasswordRequired':
                console.log(tournamentView);
                if (state != "require_password" && tournamentView != null) {
                    invoke("get_settings").then((msg) => {
                        let remote = msg.known_remotes.find(
                            (remote) => remote.url == tournamentView.remote_url
                        );
                        if (remote) {
                            setDefaultUserId(remote.account_id || "");
                        }
                        setShowLogin(true);
                    });
                }
                setState("require_password");
                break;

            default: setState("no_connection")
        }
    }

    useEffect(() => {
        invoke("get_tournament_connectivity_status", {"tournamentId": tournamentId}).then((status) => {
            if (tournamentView != null) {
                updateWithStatus(status);
            }
        });

        const unlisten = listen('connectivity-update', (event) => {
            if (event.payload.tournament_id == tournamentId && tournamentView != null) {
                updateWithStatus(event.payload);
            }
        });

        let interval = setInterval(
            () => {
                let now = new Date();
                let timePassedSinceUpdate = now - timestampRef.current;

                if (timePassedSinceUpdate > UPDATE_TIMEOUT_MILLIS) {
                    setShowDelayWarning(true)
                }
                else {
                    setShowDelayWarning(false)
                }
            }, 1
        )

        return () => {
            unlisten.then(unlisten => unlisten())
            clearInterval(interval);
        }
    }, [tournamentId, tournamentView]);
        

    if (tournamentId === null) {
        return <div></div>;
    }

    return <div>
        <ModalOverlay open={showLogin}>
            <LoginWidget
                defaultUserName={defaultUserId}
                url={tournamentView !== null ? tournamentView.remote_url: ""}
                onLogin={(username, password) => {
                    invoke("login_to_remote", {
                        remoteUrl: tournamentView.remote_url,
                        userName: username,
                        password: password,
                    }).then((msg) => {
                        setShowLogin(false);
                    }).catch(
                        (err) => {
                            setLoginError(err);
                        }
                    );
                }}
                onAbort={() => {
                    setShowLogin(false);
                }}
                loginError={loginError}
                onAccountCreation={(username, password) => {
                    invoke("create_user_account_for_remote", {
                        userName: username,
                        password: password,
                        remoteUrl: tournamentView.remote_url,
                    }).then((msg) => {
                        setShowLogin(false);
                    }).catch((err) => {
                        setLoginError(err);
                    });
                }}
            />
        </ModalOverlay>
        <ConnectivityStatusDrawer state={state} lastUpdate={timestamp} message={msg} showDelayWarning={showDelayWarning} />
    </div>
}

export default ConnectivityStatus;