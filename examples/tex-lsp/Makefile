.PHONY: help setup build clean

help:
	@echo "ggen project - Available targets:"
	@echo "  make setup   - Run startup.sh to initialize project"
	@echo "  make build   - Generate code from ontology (ggen sync)"
	@echo "  make clean   - Remove generated artifacts"
	@echo ""
	@echo "See scripts/startup.sh for custom initialization steps"

setup:
	@bash scripts/startup.sh

build:
	ggen sync

clean:
	rm -rf .ggen/
