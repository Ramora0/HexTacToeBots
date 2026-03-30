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

build: setup ## Build all C++/Rust bots and install JS dependencies
	@for setup_py in bots/*/setup.py; do \
		[ -f "$$setup_py" ] || continue; \
		dir=$$(dirname "$$setup_py"); \
		echo "Building $$dir ..."; \
		cd "$$dir" && $(CURDIR)/$(VENV)/bin/python setup.py build_ext --inplace && cd $(CURDIR); \
	done
	@for cargo_toml in bots/*/Cargo.toml; do \
		[ -f "$$cargo_toml" ] || continue; \
		dir=$$(dirname "$$cargo_toml"); \
		echo "Building $$dir (Rust) ..."; \
		cd "$$dir" && cargo build --release && cd $(CURDIR); \
	done
	@for pkg_json in bots/*/package.json; do \
		[ -f "$$pkg_json" ] || continue; \
		dir=$$(dirname "$$pkg_json"); \
		echo "Installing JS deps in $$dir ..."; \
		cd "$$dir" && npm install --silent && cd $(CURDIR); \
	done

rebuild: setup ## Clean and rebuild all C++/Rust bots from scratch
	@for setup_py in bots/*/setup.py; do \
		dir=$$(dirname "$$setup_py"); \
		echo "Rebuilding $$dir ..."; \
		rm -rf "$$dir"/build "$$dir"/*.so "$$dir"/*.pyd "$$dir"/*.egg-info; \
		cd "$$dir" && $(CURDIR)/$(VENV)/bin/python setup.py build_ext --inplace && cd $(CURDIR); \
	done
	@for cargo_toml in bots/*/Cargo.toml; do \
		dir=$$(dirname "$$cargo_toml"); \
		echo "Rebuilding $$dir (Rust) ..."; \
		cd "$$dir" && cargo clean && cargo build --release && cd $(CURDIR); \
	done

new: ## Create a new bot: make new BOT=MyBot [LANG=py|cpp|js]
	@if [ -z "$(BOT)" ]; then echo "Usage: make new BOT=MyBot [LANG=py|cpp|js|rust]"; exit 1; fi
	@if [ -d "bots/$(BOT)" ]; then echo "bots/$(BOT) already exists"; exit 1; fi
	@lang=$(or $(LANG),py); \
	if [ "$$lang" = "js" ]; then \
		cp -r examples/js_example bots/$(BOT); \
	elif [ "$$lang" = "cpp" ]; then \
		cp -r examples/cpp_example bots/$(BOT); \
	elif [ "$$lang" = "rust" ]; then \
		cp -r examples/rust_example bots/$(BOT); \
	else \
		cp -r examples/python_example bots/$(BOT); \
	fi
	@echo "Created bots/$(BOT)/ -- edit it and run: make run A=$(BOT) B=random_bot"

run: setup ## Run a match: make run A=SealBot B=random_bot [N=20] [T=0.1]
	@if [ -z "$(A)" ] || [ -z "$(B)" ]; then echo "Usage: make run A=BotA B=BotB [N=20] [T=0.1]"; exit 1; fi
	$(VENV)/bin/python evaluate.py $(A) $(B) -n $(or $(N),20) -t $(or $(T),0.1)

list: setup ## List available bots
	$(VENV)/bin/python evaluate.py --list

clean: ## Remove build artifacts and venv
	rm -rf $(VENV) __pycache__ bots/*/__pycache__ bots/*/build bots/*/*.egg-info
	find bots -name '*.so' -delete
	find bots -name '*.pyd' -delete
	find bots -name 'node_modules' -type d -exec rm -rf {} + 2>/dev/null || true
	@for cargo_toml in bots/*/Cargo.toml; do \
		[ -f "$$cargo_toml" ] || continue; \
		dir=$$(dirname "$$cargo_toml"); \
		cd "$$dir" && cargo clean 2>/dev/null; cd $(CURDIR); \
	done
