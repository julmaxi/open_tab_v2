<script>
export let data;
</script>


<nav>
	<a href="/tournament/{data.tournamentId}/home">Overview</a>
	<a href="/tournament/{data.tournamentId}/tab">Tab</a>
  <slot name="header" />
</nav>

<div class="pl-0 pr-0 md:pl-24 md:pr-24">
  <slot />
</div>
