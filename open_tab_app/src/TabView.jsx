import React, { Children, useContext } from 'react';
import { TournamentContext } from './TournamentContext';
import { useView } from './View';
import { TabGroup, Tab } from './TabGroup';


const avgPointFormat = new Intl.NumberFormat("en-US", {minimumFractionDigits: 2, maximumFractionDigits: 2});


function ScoreTable({descriptionName, numRounds, children}) {
    return <div className='overflow-scroll w-full'>
        <table className='w-full'>
            <thead className='bg-white sticky top-0'>
                <tr>
                    <th className='text-center'>#</th>
                    <th>{descriptionName}</th>
                    <th className='text-center'>Avg.</th>
                    {[...Array(numRounds).keys()].map((round) => <th className='text-center' key={round}>R{round + 1}</th>)}
                </tr>
            </thead>
            <tbody className='overflow-scroll'>
                {children} 
            </tbody>
        </table>
    </div>
}

function TeamTab({tab, numRounds}) {
    return <ScoreTable numRounds={numRounds} descriptionName={"Team"}>
            {tab.map((team) => <tr key={team.team_uuid}>
                <td className='text-center'>{team.rank + 1}</td>
                <td>{team.team_name}</td>
                <td className='text-center'>{team.avg_points == null ? "–" : avgPointFormat.format(team.avg_points) }</td>

                {
                    team.detailed_scores.map((details, idx) => <td className='text-center' key={idx}>{details !== null ? <TeamScoreDetailCell details={details} /> : "–"}</td>)
                }
            </tr>)}
    </ScoreTable>
}


function SpeakerTab({tab, numRounds}) {
    return <ScoreTable numRounds={numRounds} descriptionName={"Speaker"}>
            {tab.map((speaker) => <tr key={speaker.speaker_uuid}>
                <td className='text-center'>{speaker.rank + 1}</td>
                <td>{speaker.speaker_name}</td>
                <td className='text-center'>{speaker.avg_points == null ? "–" : avgPointFormat.format(speaker.avg_points) }</td>
                {
                    speaker.detailed_scores.map((details, idx) => <td className='text-center' key={idx}>{details !== null ? <SpeakerScoreDetailCell details={details} /> : "–"}</td>)
                }
            </tr>)}
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


function TabView() {
    let tournament = useContext(TournamentContext);

    let tabView = useView({type: "Tab", tournament_uuid: tournament.uuid}, null);
    return (
        <div className='flex w-full h-full'>
            <div className='flex-1 h-full flex flex-col'>
                <TabHeader>Teams</TabHeader>
                {
                    tabView == null ? <div>Loading...</div> : <TeamTab tab={tabView.team_tab} numRounds={tabView.num_rounds} />
                    
                }
            </div>
            <div className='flex-1 h-full flex flex-col'>
                <TabHeader>Speakers</TabHeader>
                {
                    tabView == null ? <div>Loading...</div> : <SpeakerTab tab={tabView.speaker_tab} numRounds={tabView.num_rounds} />
                }
            </div>
        </div>
    )
}

function TabsView() {
    return <TabGroup>
        <Tab name="Overview" autoScroll={false}>
            <TabView />
        </Tab>
    </TabGroup>
}


export function TabViewRoute() {
    return <TabsView />
}