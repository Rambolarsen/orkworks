<script setup>
import { withBase } from "vitepress";
import { data } from "../site.data.mts";
import DownloadLinks from "./DownloadLinks.vue";
</script>
<template>
  <div class="landing">
    <section class="site-hero" aria-labelledby="hero-title">
      <div class="hero-copy">
        <p class="eyebrow">
          <span class="status-dot"></span> YOUR CODING TOOLS. ONE WORKSPACE.
        </p>
        <h1 id="hero-title">
          More agents.<br /><span>Less tab chasing.</span>
        </h1>
        <p class="hero-description">
          Keep your AI coding sessions in view. See what’s working, what needs
          you, and where to pick up next.
        </p>
        <div class="hero-actions">
          <a
            class="site-button primary"
            :href="withBase('/docs/user/getting-started')"
            >Get started <span aria-hidden="true">↗</span></a
          ><a class="text-link" href="#how-it-works"
            >Take a look <span aria-hidden="true">↓</span></a
          >
        </div>
        <p class="hero-note">
          Local-first desktop app · Early alpha · Bring your own coding tools
        </p>
      </div>
      <div class="hero-index" aria-hidden="true">
        <span>01 / OBSERVE</span><span>02 / SWITCH</span
        ><span>03 / DECIDE</span>
      </div>
    </section>
    <figure
      class="session-preview"
      aria-label="Illustration of one focused session beside a session overview"
    >
      <div class="preview-toolbar">
        <span class="preview-logo">o<span>↗</span></span
        ><strong>OrkWorks</strong
        ><span class="preview-workspace">workspace / storefront</span
        ><span class="preview-live">● LOCAL WORKSPACE</span>
      </div>
      <div class="preview-body">
        <div class="preview-sessions">
          <p class="preview-label">SESSIONS <span>03</span></p>
          <div class="preview-session selected">
            <span class="session-state waiting">NEEDS YOU</span
            ><strong>Checkout flow</strong
            ><small>Claude Code · checkout-flow</small>
          </div>
          <div class="preview-session">
            <span class="session-state working">WORKING</span
            ><strong>Search performance</strong
            ><small>Codex · search-index</small>
          </div>
          <div class="preview-session">
            <span class="session-state done">DONE</span
            ><strong>Update documentation</strong
            ><small>GitHub Copilot CLI · docs</small>
          </div>
          <div class="preview-session-note">
            One place to see<br />what needs your attention.
          </div>
        </div>
        <div class="preview-terminal">
          <div class="terminal-tabs">
            <span>Terminal</span><span>Review</span>
          </div>
          <div class="terminal-content">
            <p class="terminal-muted">~/storefront · checkout-flow</p>
            <p>
              <span class="terminal-prompt">❯</span> Add validation to the
              checkout form.
            </p>
            <p class="terminal-muted">
              The form now checks required fields before submission.<br />Added
              tests for missing email and delivery address.
            </p>
            <p class="terminal-success">✓ Validation tests passed</p>
            <p class="terminal-question">
              Should delivery instructions be optional?
            </p>
            <span class="terminal-cursor" aria-hidden="true"></span>
          </div>
        </div>
        <div class="preview-detail">
          <p class="preview-label">SESSION DETAILS</p>
          <span class="session-state waiting">NEEDS YOUR INPUT</span>
          <h3>A decision, not another tab.</h3>
          <p>
            See the session’s context, then jump straight back into the
            conversation.
          </p>
          <hr />
          <small>CODING TOOL</small><strong>Claude Code</strong
          ><small>BRANCH</small><strong>checkout-flow</strong>
        </div>
      </div>
      <figcaption>
        Illustrative session content · One active terminal, with the rest of
        your sessions in view.
      </figcaption>
    </figure>
    <section class="tools-strip" aria-label="Built-in coding tools">
      <p>KEEP THE TOOLS YOU ALREADY USE</p>
      <ul>
        <li v-for="tool in data.tools" :key="tool.id">{{ tool.name }}</li>
      </ul>
      <a :href="withBase('/docs/user/coding-tools')"
        >Explore coding-tool support ↗</a
      >
    </section>
    <section
      id="how-it-works"
      class="benefits"
      aria-labelledby="benefits-title"
    >
      <div class="section-heading">
        <p class="eyebrow">LESS COORDINATION. MORE FLOW.</p>
        <h2 id="benefits-title">Know where your<br />attention belongs.</h2>
        <p>
          Let your coding tools do the work. Keep enough context to make the
          next decision.
        </p>
      </div>
      <div class="benefit-grid">
        <article>
          <span class="feature-number">01 — OBSERVE</span>
          <h3>Catch the moments that need you.</h3>
          <p>
            Peon, the session observer, turns terminal activity into useful
            summaries and attention signals. Spend less time checking whether an
            agent is still working.
          </p>
          <a :href="withBase('/docs/user/sessions')"
            >Understand session signals ↗</a
          >
        </article>
        <article>
          <span class="feature-number">02 — SWITCH</span>
          <h3>Pick up the thread. Keep the context.</h3>
          <p>
            Move between sessions with their terminal history, coding tool, and
            Git context close at hand. Other sessions keep running while you
            focus on one.
          </p>
          <a :href="withBase('/docs/user/sessions')">See how sessions work ↗</a>
        </article>
        <article>
          <span class="feature-number">03 — DECIDE</span>
          <h3>Turn repeated friction into a next step.</h3>
          <p>
            Taskmaster surfaces workflow improvements with supporting
            observations. Inspect the evidence, dismiss the suggestion, or
            choose Fix with AI in your active session.
          </p>
          <a :href="withBase('/docs/user/taskmaster')">Meet Taskmaster ↗</a>
        </article>
      </div>
    </section>
    <section class="ownership">
      <p class="eyebrow">YOUR WORKSPACE. YOUR CALL.</p>
      <h2>Stay in control of the work.</h2>
      <div>
        <p>
          OrkWorks hosts your sessions and helps you follow them. You choose the
          coding tool, decide what runs, and keep control of your Git workflow.
        </p>
        <p>
          Session metadata stays on your machine. Coding tools and configured AI
          observers may send context to their model providers.
          <a
            :href="
              withBase(
                '/docs/user/getting-started#your-data-and-model-providers',
              )
            "
            >How your data is used ↗</a
          >
        </p>
      </div>
    </section>
    <section class="try-section" id="try">
      <div>
        <p class="eyebrow">START WITH ONE SESSION.</p>
        <h2>Give your agents<br />a place to work.</h2>
      </div>
      <div>
        <p>
          OrkWorks is in early alpha. Try it on a project, connect a coding
          tool, and tell us what gets in your way.
        </p>
        <DownloadLinks /><a
          class="site-button primary"
          :href="withBase('/docs/user/getting-started')"
          >Set up OrkWorks <span aria-hidden="true">↗</span></a
        >
      </div>
    </section>
    <footer class="site-footer">
      <span>OrkWorks — local-first mission control.</span
      ><a href="https://github.com/Rambolarsen/orkworks/issues"
        >Feedback &amp; issues ↗</a
      ><a :href="withBase('/docs/developers')">Developer documentation ↗</a>
    </footer>
  </div>
</template>
