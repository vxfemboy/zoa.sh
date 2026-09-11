// Live search + tag filtering for /blog. Progressive enhancement: without JS
// the page still shows every post (no server-side filtering to fall back to).
(function () {
    const dataEl = document.getElementById('blog-search-data');
    const tagsEl = document.getElementById('blog-all-tags');
    if (!dataEl || !tagsEl) return;

    let index;
    let allTags;
    try {
        index = JSON.parse(dataEl.textContent);
        allTags = JSON.parse(tagsEl.textContent);
    } catch (e) {
        return;
    }

    const cards = Array.from(document.querySelectorAll('.blog-post-card'));
    const searchInput = document.getElementById('blog-search-input');
    const filterToggle = document.getElementById('blog-filter-toggle');
    const filterCount = document.getElementById('blog-filter-count');
    const tagPanel = document.getElementById('blog-tag-panel');
    const tagSearchInput = document.getElementById('blog-tag-search-input');
    const tagPills = document.getElementById('blog-tag-pills');
    const tagExpandBtn = document.getElementById('blog-tag-expand');
    const tagExpandCount = document.getElementById('blog-tag-expand-count');
    const tagClearBtn = document.getElementById('blog-tag-clear');
    const resultsCount = document.getElementById('blog-results-count');
    const emptyState = document.getElementById('blog-empty-state');

    if (!searchInput || !filterToggle || !tagPanel) return;

    const TAG_LIMIT = 8;
    const selectedTags = new Set();
    let tagsExpanded = false;
    let tagFilterQuery = '';

    // Preselect a tag from ?tag=... (deep links from old links / RSS still work).
    const params = new URLSearchParams(window.location.search);
    const initialTag = params.get('tag');
    if (initialTag && allTags.includes(initialTag)) {
        selectedTags.add(initialTag);
        // If it falls outside the default-visible slice, expand up front so
        // the pill shows as active instead of hiding under "+N more".
        if (allTags.indexOf(initialTag) >= TAG_LIMIT) {
            tagsExpanded = true;
        }
    }

    function updateFilterCount() {
        if (selectedTags.size > 0) {
            filterCount.hidden = false;
            filterCount.textContent = String(selectedTags.size);
        } else {
            filterCount.hidden = true;
        }
    }

    function toggleTag(tag) {
        if (selectedTags.has(tag)) {
            selectedTags.delete(tag);
        } else {
            selectedTags.add(tag);
        }
        renderPills();
        updateFilterCount();
        applyFilters();
    }

    function renderPills() {
        const q = tagFilterQuery.trim().toLowerCase();
        const matching = q ? allTags.filter((t) => t.toLowerCase().includes(q)) : allTags;
        const visible = tagsExpanded ? matching : matching.slice(0, TAG_LIMIT);
        const hiddenCount = matching.length - visible.length;

        tagPills.textContent = '';
        visible.forEach((tag) => {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'tag-pill' + (selectedTags.has(tag) ? ' active' : '');
            btn.textContent = tag;
            btn.setAttribute('aria-pressed', selectedTags.has(tag) ? 'true' : 'false');
            btn.addEventListener('click', () => toggleTag(tag));
            tagPills.appendChild(btn);
        });

        if (hiddenCount > 0) {
            tagExpandBtn.hidden = false;
            tagExpandCount.textContent = String(hiddenCount);
        } else {
            tagExpandBtn.hidden = true;
        }
    }

    function applyFilters() {
        const query = searchInput.value.trim().toLowerCase();
        let visibleCount = 0;

        cards.forEach((card) => {
            const i = Number(card.dataset.index);
            const post = index[i];
            if (!post) return;
            const matchesQuery =
                query === '' ||
                post.title.toLowerCase().includes(query) ||
                post.text.toLowerCase().includes(query);
            const matchesTags =
                selectedTags.size === 0 || post.tags.some((t) => selectedTags.has(t));
            const visible = matchesQuery && matchesTags;
            card.hidden = !visible;
            if (visible) visibleCount++;
        });

        if (resultsCount) {
            resultsCount.textContent =
                visibleCount === cards.length
                    ? `${cards.length} post${cards.length === 1 ? '' : 's'}`
                    : `Showing ${visibleCount} of ${cards.length} post${cards.length === 1 ? '' : 's'}`;
        }
        if (emptyState) emptyState.hidden = visibleCount !== 0;
    }

    searchInput.addEventListener('input', applyFilters);

    filterToggle.addEventListener('click', () => {
        const expanded = filterToggle.getAttribute('aria-expanded') === 'true';
        filterToggle.setAttribute('aria-expanded', String(!expanded));
        tagPanel.hidden = expanded;
    });

    tagSearchInput.addEventListener('input', () => {
        tagFilterQuery = tagSearchInput.value;
        renderPills();
    });

    tagExpandBtn.addEventListener('click', () => {
        tagsExpanded = true;
        renderPills();
    });

    tagClearBtn.addEventListener('click', () => {
        selectedTags.clear();
        tagsExpanded = false;
        renderPills();
        updateFilterCount();
        applyFilters();
    });

    renderPills();
    updateFilterCount();
    applyFilters();

    if (selectedTags.size > 0) {
        tagPanel.hidden = false;
        filterToggle.setAttribute('aria-expanded', 'true');
    }
})();
