<script>
    import { onMount } from 'svelte';
    import SlidesView from './SlidesView.svelte';
    import Slide from './Slide.svelte';
    import { get } from 'svelte/store';
  
    export let data;

    let debates = data.info.debates;
  
    let currentSlide = 0;
    console.log(data.info);
    let startTime = data.info.debate_start_time ? new Date(Date.parse(data.info.debate_start_time)) : null;

  
    onMount(() => {
      window.addEventListener('keydown', handleKeyPress);
      return () => {
        window.removeEventListener('keydown', handleKeyPress);
      };
    });
  
    function handleKeyPress(event) {
      if (event.key === 'ArrowRight' || event.key === ' ' || event.key === 'Spacebar') {
        nextSlide();
      } else if (event.key === 'ArrowLeft') {
        prevSlide();
      }
    }
  
    function nextSlide() {
      if (currentSlide < debates.length + 1) currentSlide++;
    }
  
    function prevSlide() {
      if (currentSlide > 0) currentSlide--;
    }

    function startTimer() {
        async function startTimer() {
            const response = await fetch(`./presentation/motion`, {
                method: 'POST',
                headers: {
                    'content-type': 'application/json'
                }
            });

            startTime = new Date(Date.parse((await response.json()).debate_start_time));
        }
        startTimer();
    }

    let fontSizes = {
        "title-size": "7rem",
        "header-size": "4rem",
        "subheader-size": "2.5rem",
        "teams-size": "5rem",
        "participant-size": "3rem",
        "subsubheader-size": "2.5rem",
    };

    $: cssVarStyles = Object.entries(fontSizes)
        .map(([key, value]) => `--${key}:${value}`)
        .join(';');

    function formatTime(date) {
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        return `${hours}:${minutes}`;
    }
</script>
  
<style>
    .title {
        font-size: var(--title-size);
        line-height: 1;
    }

    .header {
        font-size: var(--header-size);
        line-height: 1;
    }

    .subheader {
        font-size: var(--subheader-size);
        line-height: 1;
    }

    .team {
        font-size: var(--teams-size);
    }

    .participant {
        font-size: var(--participant-size);
        list-style-type: disc;
        list-style-position: inside;
    }

    .subheader {
        font-size: var(--subsubheader-size);
        line-height: 1;
    }
</style>

<div style={cssVarStyles} class="w-full h-screen p-4">
<SlidesView>
    <Slide slideIndex=0>
        <div style="{cssVarStyles}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full flex flex-col justify-center">
            <h1 class="title text-center">{data.info.round_name}</h1>
        </div>
    </Slide>
    {#each debates as debate, index}
    <Slide slideIndex={index + 1}>
    <div style="{cssVarStyles}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full overflow-y-auto">
        <h1 class="header font-bold p-0 mb-0">Raum {debate.debate_index + 1}</h1>
        <h2 class="subheader mb-7">{debate.venue ? debate.venue.venue_name : "" }</h2>
        <div class="grid grid-cols-2">
                <div class="text-center">
                    <p class="team font-bold">
                        {debate.government.team_name}
                    </p>
                </div>
                <div class="text-center">
                    <p class="team font-bold">
                        {debate.opposition.team_name}
                    </p>
                </div>
                <div>
                    <h3 class="subheader text-center">Freie Reden</h3>
                    <div class="flex justify-center">
                        <ul class="participant">
                            {#each debate.non_aligned_speakers as speaker}
                                <li>{speaker.participant_name}</li>
                            {/each}
                        </ul>
                    </div>
                </div>
                <div>
                    <h3 class="subheader text-center">Panel</h3>
                    <div class="flex justify-center">
                        <ul class="participant">
                            {#each debate.adjudicators as adj, adjIdx}
                                <li>
                                    {adj.participant_name} 
                                    {#if adjIdx == 0}
                                        (Chair)
                                    {/if}
                                
                                </li>
                            {/each}
                        </ul>
                    </div>
                </div>
        </div>
    </div>
    </Slide>
    {/each}
    <Slide slideIndex={debates.length + 1}>
        <div style="{cssVarStyles}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full flex flex-col justify-center">
            <h1 class="title text-center">Freie Reden bitte den Raum verlassen</h1>
        </div>
    </Slide>
    <Slide slideIndex={debates.length + 2}>
        <div style="{cssVarStyles}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full flex flex-col justify-center">
            <h1 class="title text-center">Das Thema lautet</h1>
        </div>
    </Slide>
    {#if data.info.info_slide}
    <Slide slideIndex={debates.length + 3}>
        <div style="{cssVarStyles + "; text-wrap: balance; overflow-wrap: break-word;"}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full">
            <p class="title text-center">{data.info.info_slide}</p>
        </div>
    </Slide>
    {/if}
    <Slide slideIndex={debates.length + 3 + (data.info.info_slide ? 1 : 0)} requiresReveal={true}>
        <div style="{cssVarStyles}" class="bg-white p-12 pl-20 pr-20 rounded-md shadow-md w-full h-full flex flex-col justify-center">
        <p class="title text-center">{data.info.motion}</p>

        {#if startTime !== null}
            <p class="subheader text-center">Die Debatte startet um {formatTime(startTime)}</p>
        {:else}
            <div class="w-full flex justify-center">
                <button class="text-2xl font-bold mt-4 p-4 bg-blue-500 text-white rounded" on:click={startTimer}>Publish</button>
            </div>
        {/if}
        </div>
    </Slide>
</SlidesView>
</div>
