<script>
export let data;
</script>


<nav>
	<a href="/tournament/{data.tournamentId}/home">Overview</a>
	<a href="/tournament/{data.tournamentId}/tab">Tab</a>
  <slot name="header" />
</nav>

<div class="">
  <slot />
</div>
