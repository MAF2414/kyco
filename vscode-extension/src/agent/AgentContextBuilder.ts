import type {
    VisGraph,
    MapSelection,
    SelectionContext,
    AgentPayload,
    OutputFormat,
} from '../graph/types';
import { SelectionSerializer } from './SelectionSerializer';

/**
 * AgentContextBuilder orchestrates the transformation of map selection
 * into structured context for AI agents.
 *
 * It uses SelectionSerializer to extract rich context from the visual
 * selection and serialize it to the desired output format.
 */
export class AgentContextBuilder {
    private serializer: SelectionSerializer;
    private graph: VisGraph | null = null;
    private activeLayers: string[] = [];
    private workspaceRoot: string = '';

    constructor() {
        this.serializer = new SelectionSerializer();
    }

    /**
     * Update the graph data
     */
    setGraph(graph: VisGraph): void {
        this.graph = graph;
    }

    /**
     * Update the active layers
     */
    setActiveLayers(layers: string[]): void {
        this.activeLayers = layers;
    }

    /**
     * Set the workspace root path for relative file paths
     */
    setWorkspaceRoot(root: string): void {
        this.workspaceRoot = root;
    }

    /**
     * Build context from map selection
     */
    async buildContext(
        selection: MapSelection,
        userPrompt: string,
        outputFormat: OutputFormat = 'yaml'
    ): Promise<AgentPayload> {
        if (!this.graph) {
            throw new Error('Graph not set. Call setGraph() first.');
        }

        // Serialize the selection into rich context
        const context = await this.serializer.serialize(
            selection,
            this.graph,
            this.activeLayers,
            this.workspaceRoot
        );

        // Format the context
        const serialized = this.formatContext(context, outputFormat);

        return {
            context: serialized,
            prompt: userPrompt,
            format: outputFormat,
        };
    }

    /**
     * Build context from node IDs only (legacy support)
     */
    async buildContextFromNodeIds(
        nodeIds: string[],
        userPrompt: string,
        outputFormat: OutputFormat = 'yaml'
    ): Promise<AgentPayload> {
        const selection: MapSelection = {
            nodeIds,
            edgeIds: [],
            viewport: { x: 0, y: 0, scale: 1 },
        };

        return this.buildContext(selection, userPrompt, outputFormat);
    }

    /**
     * Get the raw SelectionContext without formatting
     */
    async getSelectionContext(selection: MapSelection): Promise<SelectionContext> {
        if (!this.graph) {
            throw new Error('Graph not set. Call setGraph() first.');
        }

        return this.serializer.serialize(
            selection,
            this.graph,
            this.activeLayers,
            this.workspaceRoot
        );
    }

    /**
     * Format context to the specified output format
     */
    formatContext(context: SelectionContext, format: OutputFormat): string {
        switch (format) {
            case 'yaml':
                return this.serializer.toYAML(context);
            case 'json':
                return this.serializer.toJSON(context);
            case 'markdown':
                return this.serializer.toMarkdown(context);
            default:
                return this.serializer.toYAML(context);
        }
    }

    /**
     * Create a complete agent payload with context and prompt
     */
    createPayload(
        serializedContext: string,
        userPrompt: string,
        format: OutputFormat
    ): AgentPayload {
        return {
            context: serializedContext,
            prompt: userPrompt,
            format,
        };
    }

    /**
     * Get the serializer instance for direct access
     */
    getSerializer(): SelectionSerializer {
        return this.serializer;
    }
}
