<script>
    export let score;
    export let header;

    function formatScore(score) {
        return isNaN(score) ? "-" : score.toFixed(2);
    }
</script>


<style>
    .score {
        font-size: 1.5rem;
        font-weight: bold;
    }
    .header {
        font-size: 0.9rem;
        font-weight: normal;
        color: rgb(107 114 128);
        text-align: center;
    }
    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: flex-start;
        padding: 0.5rem;
        background-color: rgb(251, 250, 254);

        flex: 1;
    }
</style>
<div class="container">
    <span class="score">{formatScore(score)}</span>
    <h2 class="header">{header}</h2>
</div>