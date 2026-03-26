PYTHON ?= python3
VENV   := .venv

.PHONY: help setup build rebuild clean new run list

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-12s %s\n", $$1, $$2}'

setup: $(VENV)/bin/activate ## Create venv and install dependencies
$(VENV)/bin/activate: requirements.txt
	$(PYTHON) -m venv $(VENV)
	$(VENV)/bin/pip install -r requirements.txt
	@touch $@

build: setup ## Build all C++ bots (incremental)
	@for setup_py in bots/*/setup.py; do \
		dir=$$(dirname "$$setup_py"); \
		echo "Building $$dir ..."; \
		cd "$$dir" && $(CURDIR)/$(VENV)/bin/python setup.py build_ext --inplace && cd $(CURDIR); \
	done

rebuild: setup ## Clean and rebuild all C++ bots from scratch
	@for setup_py in bots/*/setup.py; do \
		dir=$$(dirname "$$setup_py"); \
		echo "Rebuilding $$dir ..."; \
		rm -rf "$$dir"/build "$$dir"/*.so "$$dir"/*.pyd "$$dir"/*.egg-info; \
		cd "$$dir" && $(CURDIR)/$(VENV)/bin/python setup.py build_ext --inplace && cd $(CURDIR); \
	done

new: ## Create a new bot: make new BOT=MyBot
	@if [ -z "$(BOT)" ]; then echo "Usage: make new BOT=MyBot"; exit 1; fi
	@if [ -d "bots/$(BOT)" ]; then echo "bots/$(BOT) already exists"; exit 1; fi
	mkdir -p bots/$(BOT)
	cp examples/example.py bots/$(BOT)/bot.py
	touch bots/$(BOT)/__init__.py
	@sed -i '' 's/my_bot/$(BOT)/g' bots/$(BOT)/bot.py 2>/dev/null || \
		sed -i 's/my_bot/$(BOT)/g' bots/$(BOT)/bot.py
	@echo "Created bots/$(BOT)/bot.py -- edit it and run: make run A=$(BOT) B=random_bot"

run: setup ## Run a match: make run A=SealBot B=random_bot [N=20] [T=0.1]
	@if [ -z "$(A)" ] || [ -z "$(B)" ]; then echo "Usage: make run A=BotA B=BotB [N=20] [T=0.1]"; exit 1; fi
	$(VENV)/bin/python evaluate.py $(A) $(B) -n $(or $(N),20) -t $(or $(T),0.1)

list: setup ## List available bots
	$(VENV)/bin/python evaluate.py --list

clean: ## Remove build artifacts and venv
	rm -rf $(VENV) __pycache__ bots/*/__pycache__ bots/*/build bots/*/*.egg-info
	find bots -name '*.so' -delete
	find bots -name '*.pyd' -delete
