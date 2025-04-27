import React, { Children, useContext } from 'react';
import { TournamentContext } from './TournamentContext';
import { useView } from './View';
import { TabGroup, Tab } from './TabGroup';
import { save } from '@tauri-apps/api/dialog';
import { invoke } from '@tauri-apps/api/tauri';


const avgPointFormat = new Intl.NumberFormat("en-US", {minimumFractionDigits: 2, maximumFractionDigits: 2});


function ScoreTable({descriptionName, numRounds, children}) {
    return <div className='overflow-auto w-full relative'>
        <table className='w-full'>
            <thead className='bg-white sticky top-0'>
                <tr>
                    <th className='text-center'>#</th>
                    <th>{descriptionName}</th>
                    <th className='text-center'>Avg.</th>
                    {[...Array(numRounds).keys()].map((round) => <th className='text-center' key={round}>R{round + 1}</th>)}
                </tr>
            </thead>
            <tbody>
                {children} 
            </tbody>
        </table>
    </div>
}


function TeamTabRow({team, markedBreaks}) {
    return <tr key={team.team_uuid}>
        <td className='text-center'>{team.rank + 1}</td>
        <td>{team.team_name}{markedBreaks.length > 0 ? (<p className='text-xs'>{markedBreaks.join(", ")}</p>) : []}</td>
        <td className='text-center'>{team.avg_score == null ? "–" : avgPointFormat.format(team.avg_score) }</td>

        {
            team.detailed_scores.map((details, idx) => <td className='text-center' key={idx}>{details !== null ? <TeamScoreDetailCell details={details} /> : "–"}</td>)
        }
    </tr>
}

function TeamTab({tab, numRounds, markedBreaks}) {
    return <ScoreTable numRounds={numRounds} descriptionName={"Team"}>
        {
            tab.map(
                (team, idx) => {
                    let teamMarkedBreaks = Object.keys(markedBreaks).filter((key) => markedBreaks[key].includes(team.team_uuid));
                    return <TeamTabRow key={idx} team={team} markedBreaks={teamMarkedBreaks} />
                }
            )
        }
    </ScoreTable>
}


function SpeakerTab({tab, numRounds, markedBreaks}) {
    return <ScoreTable numRounds={numRounds} descriptionName={"Speaker"}>
        {
            tab.map((speaker) => {
                let speakerMarkedBreaks = Object.keys(markedBreaks).filter((key) => markedBreaks[key].includes(speaker.speaker_uuid));
                return <tr key={speaker.speaker_uuid}>
                    <td className='text-center'>{speaker.rank + 1}</td>
                    <td>
                        {speaker.speaker_name}
                        {speakerMarkedBreaks.length > 0 ? (<p className='text-xs'>{speakerMarkedBreaks.join(", ")}</p>) : []}    
                    </td>
                    <td className='text-center'>{speaker.avg_score == null ? "–" : avgPointFormat.format(speaker.avg_score) }</td>
                    {
                        speaker.detailed_scores.map((details, idx) => <td className='text-center' key={idx}>{details !== null ? <SpeakerScoreDetailCell details={details} /> : "–"}</td>)
                    }
                </tr>
            }
        )
    }
    </ScoreTable>
}

function roleToIndicator(role) {
    switch (role) {
        case "Government":
            return "G";
        case "Opposition":
            return "O";
        case "NonAligned":
            return "N";
    }
}

function TeamScoreDetailCell({details}) {
    let bgColor = "bg-gray-200";
    switch (details.role) {
        case "Government":
            bgColor = "bg-green-200";
            break;
        case "Opposition":
            bgColor = "bg-orange-200";
            break;
        case "NonAligned":
            bgColor = "bg-violet-200";
            break;
    };
    return <div>
        <div>{avgPointFormat.format(details.speaker_score + (details.team_score  || 0))}</div>
        <div className={`text-xs ${bgColor} rounded-b pl-1 pr-1`}>
            {roleToIndicator(details.role) }
            {
                details.role === "NonAligned" ? [] :
                <span className='pl-1'>
                    {avgPointFormat.format(details.speaker_score)}
                    +
                    {details.team_score !== null ? avgPointFormat.format(details.team_score) : "–"}
                </span>
            }
        </div>
    </div>
}

function SpeakerScoreDetailCell({details}) {
    let bgColor = "bg-gray-200";
    switch (details.team_role) {
        case "Government":
            bgColor = "bg-green-200";
            break;
        case "Opposition":
            bgColor = "bg-orange-200";
            break;
        case "NonAligned":
            bgColor = "bg-violet-200";
            break;
    };
    return <div>
        <div>{avgPointFormat.format(details.score)}</div>
        <div className={`text-xs ${bgColor} rounded-b pl-1 pr-1`}>
            { roleToIndicator(details.team_role) }{ details.speech_position + 1 }
        </div>
    </div>
}


function TabHeader({children}) {
    return <h1 className='font-bold text-lg'>
        {children}
    </h1>
}


function CurrentTabView() {
    let tournament = useContext(TournamentContext);

    let tabView = useView({type: "Tab", tournament_uuid: tournament.uuid}, null);
    return <TabView tabView={tabView} />
}

function TabView({tabView, breakingTeams: breakingTeams = [], breakingSpeakers: breakingSpeakers = [], teamBreakingSpeakers: teamBreakingSpeakers = [], breakNodeId: breakNodeId = null}) {
    let tournamentId = useContext(TournamentContext).uuid;
    return (
        <div className='flex flex-1 w-full h-full flex-col'>
            <div className='flex flex-1 w-full min-h-0'>
                <div className='flex-1 flex flex-col max-w-[50%]'>
                    <TabHeader>Teams</TabHeader>
                    {
                        tabView == null ? <div>Loading...</div> : <TeamTab tab={tabView.team_tab} numRounds={tabView.num_rounds} markedBreaks={
                            {"Break": breakingTeams}
                        } />
                    }
                </div>
                <div className='flex-1 h-full flex flex-col max-w-[50%]'>
                    <TabHeader>Speakers</TabHeader>
                    {
                        tabView == null ? <div>Loading...</div> : <SpeakerTab tab={tabView.speaker_tab} numRounds={tabView.num_rounds} markedBreaks={
                            {"Break": breakingSpeakers, "Break in Team": teamBreakingSpeakers}
                        } />
                    }
                </div>
            </div>
            <div className="flex-none w-full h-12 bg-gray-200">
                <button onClick={() => {
                    save({defaultPath: "tab.odt", filters: [{name: "odt", extensions: ["odt"]}]}).then(
                        selected => {
                            if (selected != null) {
                                invoke("save_tab", {path: selected, nodeId: breakNodeId, tournamentId: tournamentId});
                            }
                        }
                    )
                }} className="h-full">Export as OpenOffice Doc…</button>
            </div>
        </div>
    )
}

function BreakTabView({breakNodeId}) {
    let tabView = useView({type: "BreakRelevantTab", node_uuid: breakNodeId}, null);
    let teamBreakingSpeakers = [];

    for (let team of tabView?.breaking_teams || []) {
        teamBreakingSpeakers.push(...tabView.team_members[team]);
    }

    return tabView ? <TabView tabView={tabView.tab} breakingSpeakers={tabView.breaking_speakers} teamBreakingSpeakers={teamBreakingSpeakers} breakingTeams={tabView.breaking_teams} breakNodeId={breakNodeId} /> : <div>Loading...</div>;
}

function TabsView() {
    let tournamentUuid = useContext(TournamentContext).uuid;
    let breaks = useView({type: "Breaks", tournament_uuid: tournamentUuid}, null);

    return <TabGroup>
        <Tab name="Overview" autoScroll={false}>
            <CurrentTabView />
        </Tab>
        {
            (breaks?.breaks || []).map(
                (break_, idx) => {
                return <Tab name={break_.name} key={break_.node_id} autoScroll={false}>
                    <BreakTabView breakNodeId={break_.node_id} />
                </Tab>
                }
            )
        }
    </TabGroup>
}


export function TabViewRoute() {
    return <TabsView />
}