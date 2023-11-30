import { useTransition, animated, useSpringRef } from '@react-spring/web'
import { useState, useEffect, useMemo, useContext } from 'react';
import { TournamentContext } from './TournamentContext';
import Button from './Button';
import { useView } from './View';
import RemoteSelector from './RemoteSelector';
import { useSettings } from './settings';
import { invoke } from '@tauri-apps/api/tauri';
import { openImportDialog } from './openImportDialog';
import { ParticipantImportDialogButton } from './ParticipantImportDialog';
import { Link as RouterLink } from 'react-router-dom';
import { executeAction } from './Action';
import { open, save } from '@tauri-apps/api/dialog';
import { DateTimeSelectorButton } from './DateTimeSelectorButton';
import { AdjudicatorBreakSelector } from './AdjudicatorBreakSelector';
import ModalOverlay from './Modal';

const StepTypeRenderers = {
    "LoadParticipants": LoadParticipantsStep,
    "WaitForDraw": WaitForDrawStep,
    "Welcome": WelcomeStep,
    "WaitForPublishRound": WaitForPublishRoundStep,
    "WaitForMotionRelease": WaitForMotionRelease,
    "WaitForResults": WaitForResultsStep,
    "WaitForBreak": WaitForBreakStep,
    "Done": DoneStep,
};


function DoneStep({ }) {
    return <div className='w-full'>
        <h1>Done</h1>

        <p>
            Congratulations, the tournament is over.
        </p>
    </div>
}

function Link({ to, children }) {
    return <RouterLink to={to} className="text-blue-500 underline">{children}</RouterLink>
}

function WaitForBreakStep({ node_uuid }) {
    let tournamentContext = useContext(TournamentContext);
    return <div className='w-full'>
        <h1>Break</h1>

        <p>
            The tournament requires a break at this step.
            Check the results of the previous rounds and make sure, that the break is correct.
            If you need to break manually, you can do so in the <Link to="/rounds">rounds overview</Link>.
            Click on the node in the overview and push the button in the side panel.
        </p>

        <Button onClick={() => {
            executeAction("ExecutePlanNode", { plan_node: node_uuid, tournament_id: tournamentContext.uuid });
        }} role="primary">Break</Button>
    </div>
}


function WaitForResultsStep({ round_uuid, num_submitted, num_expected }) {
    let isDone = num_submitted >= num_expected;
    return <div className='w-full'>
        <h1>Results</h1>

        <p>Adjudicators can now submit ballots.</p>
        <p>
            You can also enter the results of the round manually in the <Link to={`/round/${round_uuid}/results`}>results view</Link>.
        </p>

        {isDone ? <p>
            Once all ballots are here, you can continue with the tournament by closing the round.
            You can also close the round, before all ballots are in.
        </p> : []}

        <p>
            {num_submitted} of {num_expected} ballots are in.
        </p>

        <DateTimeSelectorButton
            buttonFactory={Button}
            buttonProps={{ role: (isDone ? "primary" : "secondary") }}
            label="Set Round End Time"
            onSetDate={(date) => {
                if (date !== null) {
                    executeAction("UpdateRound", {
                        round_id: round_uuid, update: {
                            "round_close_time": date.toISOString().slice(0, -1),
                        }
                    });
                }
            }}
        >
            Release Draw…
        </DateTimeSelectorButton>
    </div>
}

function WaitForMotionRelease({ round_uuid }) {
    return <div className='w-full'>
        <h1>Release Motion</h1>

        <p>Once you are done with the presentation, you can release the motion to all adjudicators and participants that are not non-aligned.</p>
        <p>
            This will also set the debate start time to 15 minutes and the release of the motion to non-aligned speakers to 20 minutes after you click the button.
            If you want to override this behavior, you can do so in the <Link to={`/round/${round_uuid}/publish`}>publication view</Link>.
        </p>
        <p>If you use the online presentation, the times will be scheduled automatically.</p>


        <DateTimeSelectorButton
            buttonFactory={Button}
            buttonProps={{ role: "primary" }}
            label="Set Release Time"
            onSetDate={(date) => {
                if (date !== null) {
                    let debateStartTime = new Date(date.getTime() + 15 * 60000);
                    let fullMotionReleaseTime = new Date(date.getTime() + 20 * 60000);

                    executeAction("UpdateRound", {
                        round_id: round_uuid, update: {
                            "debate_start_time": debateStartTime.toISOString().slice(0, -1),
                            "team_motion_release_time": date.toISOString().slice(0, -1),
                            "full_motion_release_time": fullMotionReleaseTime.toISOString().slice(0, -1),
                        }
                    });
                }
            }}
        >
            Release Draw…
        </DateTimeSelectorButton>

    </div>
}


function WaitForPublishRoundStep({ round_uuid }) {
    let tournamentContext = useContext(TournamentContext);
    return <div className='w-full'>
        <h1>Publish Round</h1>
        <p>
            You are now ready to publish the draw of the round.
            Before you continue, you should <Link to={`/round/${round_uuid}/draw`}>check the draw</Link> and make sure, that it is correct.
            You can initiate the release clicking the button below.
            All releases follow a time-based system, so you can
            preschedule the draw release for a specific time.

            You can save the draw presentation, along with the print-out ballots
            by clicking the button below.
            Note that ig you do not have the online functionality enabled, you should still set release times to proceed. They will have no other effect.
        </p>
        <Button role="secondary" onClick={
            () => {
                open({ directory: true }).then((result) => {
                    invoke("save_round_files", { roundId: round_uuid, dirPath: result }).then((result) => {
                        console.log(result);
                    });
                });
            }
        }>Export Ballots/Presentation…</Button>

        <DateTimeSelectorButton
            buttonFactory={Button}
            buttonProps={{ role: "primary" }}
            label="Set Release"
            onSetDate={(date) => {
                executeAction("UpdateRound", { round_id: round_uuid, update: { "draw_release_time": date === null ? null : date.toISOString().slice(0, -1) } });
            }}
        >
            Release Draw…
        </DateTimeSelectorButton>
    </div>
}

function WelcomeStep({ }) {
    return <div className='w-full'>
        <h1>Welcome</h1>

        <p>
            Welcome to the Tab-Assistant. It will do the best
            to guide you through this tournament. However, it
            will only work for tournaments that follow a certain standard formula.
            As long as you only follow the instructions here, it should work fine.
            If you have specicial requirements, all functionalities here (and more) are availble in the
            side bar and you can ignore this pane entirely.
        </p>
    </div>
}

function LoadParticipantsStep({ }) {
    let tournamentContext = useContext(TournamentContext);
    let statusView = useView({ type: "TournamentStatus", tournament_uuid: tournamentContext.uuid }, null);
    let settings = useSettings();


    return <div className='w-full'>
        <h1>Load Participants</h1>

        <p>
            To get started, you need to import your particiants to the tournament.
            The easiest way is to download the <a>Example CSV</a> and follow the format there.
        </p>
        <p>
            To improve the automatic draw, you can rate the chairing and wing abilities of the adjudicators.
            Simply write the scores behind a hash mark in the role column.
            For example <span>#42</span> will give the adjudicator a chairing score of 40 and a wing score of 20.
            You can later adjust these in the <a href="/participants">participants overview</a>.
        </p>

        <p>
            If you want to make use of the online functionality, you will need to select
            a remote server to host the tournament on.
        </p>

        <div className='p-4'>
            {
                settings && statusView &&
                <RemoteSelector
                    knownRemotes={settings.known_remotes || []}
                    currentRemoteUrl={statusView.remote_url}
                    onSetRemote={(url) => {
                        invoke("set_remote", { remoteUrl: url, tournamentId: tournamentContext.uuid });
                    }
                    }
                />
            }
        </div>

        <ParticipantImportDialogButton buttonFactory={Button} buttonProps={{ role: "primary" }} />
    </div>
}

function WaitForDrawStep({ node_uuid, is_first_in_tournament, previous_break_node }) {
    let tournamentContext = useContext(TournamentContext);
    let [isEditingAdjudicatorBreak, setIsEditingAdjudicatorBreak] = useState(false);

    return <div className='w-full'>
        <h1>Draw</h1>

        {is_first_in_tournament && <><p>
            Before you continue, you should make sure, all clashes
            and institution memberships are correct and fix them if necessary.
            You can do this in the <Link to="/participants">participants overview</Link>.
        </p>
            <p>
                You should also make sure, that the plan for your tournaments conforms to your
                expectations. You can do this in the <Link to="/rounds">rounds overview</Link>.
            </p>
        </>}

        {previous_break_node && <p>
            Since this round happens after a break, you might want to add an adjudicator draw
            before you continue. This way, only breaking adjudicators will be assigned in the next step.
            This will also show adjudicators on the tab when you export it.
            If you want to keep all adjudicators, you can ignore this step.
        </p>}

        {previous_break_node && <p>
            You might also want to release feedback for the rounds in the break.
        </p>}

        {
            previous_break_node && <p>
                Finally, if you want a printed tab, you can save it from here.
            </p>
        }



        Once you are ready, you can generate the draw for the next batch of rounds.

        <div>
            {previous_break_node && <>
                <DateTimeSelectorButton
                    buttonFactory={Button}
                    buttonProps={{ role: "secondary" }}
                    label="Release Feedback"
                    onSetDate={(date) => {
                        if (date !== null) {
                            executeAction("SetBreakRelease", {
                                node_uuid: previous_break_node,
                                time: date.toISOString().slice(0, -1),
                            });
                        }
                    }}
                >
                Release Draw…
            </DateTimeSelectorButton>
 
                <Button onClick={
                    () => {
                        save({ defaultPath: "tab.odt", filters: [{ name: "odt", extensions: ["odt"] }] }).then(
                            selected => {
                                if (selected != null) {
                                    invoke("save_tab", { path: selected, nodeId: node_uuid, tournamentId: tournamentContext.uuid });
                                }
                            }
                        )
                    }
                }>
                    Save Break Tab…
                </Button>


                <Button onClick={
                    () => {
                        setIsEditingAdjudicatorBreak(true);
                    }
                } role="secondary">Set Adjudicator Break…</Button>

                <ModalOverlay open={isEditingAdjudicatorBreak} windowClassName="flex h-screen">
                    {isEditingAdjudicatorBreak ? <AdjudicatorBreakSelector nodeId={previous_break_node} onAbort={
                        () => {
                            setIsEditingAdjudicatorBreak(false);
                        }
                    } onSuccess={
                        (breakingAdjudicators) => {
                            executeAction(
                                "SetAdjudicatorBreak",
                                {
                                    node_id: previous_break_node,
                                    breaking_adjudicators: breakingAdjudicators,
                                }
                            )

                            setIsEditingAdjudicatorBreak(false);
                        }

                    } /> : []}
                </ModalOverlay>


            </>}

            <Button onClick={() => {
                executeAction("ExecutePlanNode", { plan_node: node_uuid, tournament_id: tournamentContext.uuid });
            }} role="primary">Generate Draw</Button>
        </div>
    </div>
}

function AssistantStepPane({ children, isActive = false }) {
    let className = "w-full p-4 pl-10 flex-col border-b";
    if (!isActive) {
        className += " text-gray-500";
    }
    return <div className={className}>
        <div>
            {children}
        </div>
    </div>
}

function AssistantLadder({ steps }) {
    let orderedSteps = [...steps];
    orderedSteps.reverse();

    const refMap = useMemo(() => new WeakMap(), [])

    const transRef = useSpringRef();
    useEffect(() => {
        transRef.start()
    }, [orderedSteps])


    const [transitions, api] = useTransition(orderedSteps, () => ({
        ref: transRef,
        from: { opacity: 0, height: 0 },
        initial: { opacity: 1 },
        enter: item => async (next, cancel) => {
            await next({ opacity: 1, height: refMap.get(item).offsetHeight })
            await next({ height: 'auto' })
        },
        config: { tension: 125, friction: 20, precision: 0.1 },
        keys: (item) => {
            return item.key;
        },
    }))

    return <div className="w-full h-full flex flex-col overflow-auto">
        <div className="w-full h-full flex flex-col">
            {
                transitions((style, step, idx) => {
                    let StepRenderer = StepTypeRenderers[step.step_type];
                    return <animated.div key={step.key} style={style}><div ref={(ref) => ref && refMap.set(step, ref)}><AssistantStepPane isActive={!step.is_done} key={idx}>
                        <StepRenderer {...step} />
                    </AssistantStepPane></div></animated.div>
                })
            }
        </div>
    </div>
}

function Assistant({ }) {
    let tournamentContext = useContext(TournamentContext);
    let state = useView({ type: "Progress", tournament_uuid: tournamentContext.uuid }, null);

    let steps = state ? ([{ "step_type": "Welcome", is_done: state.steps.length > 0 }, ...state.steps]) : [];

    steps = steps.map((s) => {
        let node_uuid = s.node_uuid;
        let round_uuid = s.round_uuid;
        let key = s.step_type;

        if (node_uuid !== undefined) {
            key += node_uuid;
        }
        if (round_uuid !== undefined) {
            key += round_uuid;
        }

        return {
            key,
            ...s
        }
    });

    return <div className="w-full h-full flex flex-col">
        {steps.length && <AssistantLadder steps={steps} />}
    </div>
}


export function AssistantRoute() {
    return (
        <Assistant />
    );
}