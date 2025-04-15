<script>
    import DistPlotWidget from './DistPlotWidget.svelte';
import HistoryWidget from './HistoryWidget.svelte';
import ScoreWidget from './ScoreWidget.svelte';

    export let data;
    const statistics = data.statistics;

    console.log(statistics);
</script>

<style>
    h1 {
        font-size: 1.5rem;
        font-weight: bold;
        margin-bottom: 1rem;
    }
    h2 {
        font-size: 1.25rem;
        font-weight: bold;
    }
    .container {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        justify-content: space-around;
        align-items: flex-start;
    }

    .award {
        display: flex;
        flex-direction: column;
        align-items: center;
    }
    
    .award > img {
        height: 125px;
        max-width: 125px;
    }

    .award > h4 {
        font-size: 1rem;
        font-weight: bold;
    }

    .award > h5 {
        font-size: 0.8rem;
        font-weight: normal;
        color: rgb(107 114 128);
    }
</style>

<h1>{data.userIdentifier}</h1>

<h1>Awards</h1>

<div class="container">
    {#each statistics.awards as award}
        <div class="award">
            <img
                src={award.image ? `/assets/${award.image}` : "/default_award.png"}
                alt="Award"
            />
            <h4>{award.tournament_name}</h4>
            <h5>{award.title}</h5>
        </div>
    {/each}
</div>

<h1>Statistics</h1>

<h2>Speaker Statistics</h2>

<div class="container">
    <ScoreWidget
        score={statistics.lifetime_average_speech_score}
        header="Speech Average"
    />
    <ScoreWidget
        score={statistics.lifetime_max_speech_score}
        header="Best Speech"
    />
    <ScoreWidget
        score={statistics.lifetime_average_team_score}
        header="Team Average"
    />
    <ScoreWidget
        score={statistics.lifetime_max_team_score}
        header="Best Team Performance"
    />
</div>

<div class="container">
    <HistoryWidget
    samples={statistics.score_samples}
    />

    <DistPlotWidget samples={statistics.score_samples} />
</div>
