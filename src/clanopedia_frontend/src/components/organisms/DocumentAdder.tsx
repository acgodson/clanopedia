import React, { useState, useCallback, useEffect } from 'react';
import { useDropzone } from 'react-dropzone';
import { useAuth } from '../../providers/useAuth';
import { useToast } from '../../providers/toast';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../atoms/tabs';
import { Card, CardContent, CardHeader, CardTitle } from '../atoms/card';
import { Button } from '../atoms/button';
import { Input } from '../atoms/input';
import { Progress } from '../atoms/progress';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '../atoms/sheet';
import { Youtube, Github, FileText, Loader2 } from 'lucide-react';
import { cn } from '../../lib/utils';
import { useNavigate } from 'react-router-dom';

interface DocumentAdderProps {
    collectionId: string;
    collectionName: string;
    onClose: () => void;
}

type ExtractionStep = 'idle' | 'extracting' | 'processing' | 'complete' | 'error';
type ExtractionSource = 'file' | 'youtube' | 'github';

interface ExtractionProgress {
    step: ExtractionStep;
    source: ExtractionSource;
    progress: number;
    message: string;
    extractedDocuments: any[];
    operationId?: string;
}

interface BatchExtractionState {
    hasMore: boolean;
    nextPageToken?: string;
    totalVideos?: number;
    processedVideos: number;
}

// New interface for extracted documents
interface ExtractedDocument {
    title: string;
    content: string;
    content_type: string;
    source_url?: string;
    author?: string;
    tags?: string[];
    isEmbedded?: boolean;
    isSelected?: boolean;
    isAdded?: boolean;
}

interface BatchExtractionState {
    hasMore: boolean;
    nextPageToken?: string;
    totalVideos?: number;
    processedVideos: number;
}

// Helper functions for URL validation
const isValidGitHubUrl = (url: string): boolean => {
    const urlLower = url.toLowerCase();
    if (!urlLower.includes('github.com')) return false;

    // Check if it's a raw markdown file
    if (urlLower.includes('raw.githubusercontent.com')) {
        return urlLower.endsWith('.md');
    }

    // Check if it's a blob URL with markdown file
    if (urlLower.includes('/blob/')) {
        return urlLower.endsWith('.md');
    }

    // If it's a repository URL, we'll try to convert it
    return urlLower.match(/^https?:\/\/github\.com\/[^\/]+\/[^\/]+$/i) !== null;
};

const convertRepoToReadmeUrl = (url: string): string => {
    // If it's already a markdown file or raw URL, return as is
    if (url.toLowerCase().endsWith('.md') || url.includes('raw.githubusercontent.com')) {
        return url;
    }

    // If it's a repository URL, convert to README.md
    if (url.match(/^https?:\/\/github\.com\/[^\/]+\/[^\/]+$/i)) {
        return `${url}/blob/main/README.md`;
    }

    return url;
};

// New component for the extraction sheet
function ExtractionSheet({
    collectionId,
    documents,
    onClose,
    onEmbed,
    batchState,
    onContinue,
    onDocumentsChange,
    navigate
}: {
    collectionId: string;
    documents: ExtractedDocument[];
    onClose: () => void;
    onEmbed: (doc: ExtractedDocument) => Promise<void>;
    batchState: BatchExtractionState;
    onContinue: () => Promise<void>;
    onDocumentsChange: (docs: ExtractedDocument[]) => void;
    navigate: (path: string) => void;
}) {
    const { ClanopediaActor, isAuthenticated, login, principal } = useAuth();
    const { toast } = useToast();
    const [embeddingDoc, setEmbeddingDoc] = useState<string | null>(null);
    const [isAddingToBlueband, setIsAddingToBlueband] = useState(false);

    const handleSelectAll = (selected: boolean) => {
        const updatedDocs = documents.map(doc => ({ ...doc, isSelected: selected }));
        onDocumentsChange(updatedDocs);
    };

    const handleSelectDocument = (index: number, selected: boolean) => {
        const updatedDocs = [...documents];
        updatedDocs[index] = { ...updatedDocs[index], isSelected: selected };
        onDocumentsChange(updatedDocs);
    };

    const handleAddSelectedToBlueband = async () => {
        const selectedDocs = documents.filter(doc => doc.isSelected);
        if (selectedDocs.length === 0 || !ClanopediaActor) return;

        try {
            console.log('Sending documents to add:', selectedDocs);
            const result = await ClanopediaActor!.add_extracted_documents(collectionId, selectedDocs);
            console.log('Received result:', result);

            if ('Ok' in result) {
                const { document_ids, proposal_id, message } = result.Ok;
                const updatedDocs = documents.map(doc => ({
                    ...doc,
                    isSelected: false,
                    isAdded: true
                }));
                onDocumentsChange(updatedDocs);
                toast({
                    title: "Documents added",
                    description: (
                        <div className="space-y-2">
                            <p>{message}</p>
                            <Button
                                variant="link"
                                className="p-0 h-auto"
                                onClick={() => navigate(`/collections/${collectionId}/proposals/${proposal_id}`)}
                            >
                                View proposal
                            </Button>
                        </div>
                    ),
                });
            } else {
                const error = result.Err;
                if (typeof error === 'object') {
                    if ('NotFound' in error && error.NotFound?.includes('Proposal')) {
                        // Handle proposal not found error with more specific message
                        const updatedDocs = documents.map(doc => ({
                            ...doc,
                            isSelected: false,
                            isAdded: true
                        }));
                        onDocumentsChange(updatedDocs);
                        toast({
                            title: "Documents added",
                            description: "Documents were added successfully, but proposal creation failed. The documents are in the collection but may need to be embedded manually.",
                        });
                    } else if ('NotAuthorized' in error) {
                        toast({
                            title: "Not authorized",
                            description: "You don't have permission to add documents to this collection.",
                            variant: "destructive",
                        });
                    } else if ('InvalidInput' in error) {
                        toast({
                            title: "Invalid input",
                            description: error.InvalidInput,
                            variant: "destructive",
                        });
                    } else {
                        throw new Error('Failed to add documents: ' + JSON.stringify(error));
                    }
                } else {
                    throw new Error(typeof error === 'string' ? error : 'Failed to add documents');
                }
            }
        } catch (error) {
            console.error('Error in handleAddSelectedToBlueband:', error);
            toast({
                title: "Error",
                description: error instanceof Error ? error.message : "Failed to add documents",
                variant: "destructive",
            });
        }
    };

    const selectedCount = documents.filter(doc => doc.isSelected).length;

    return (
        <Sheet open={true} onOpenChange={onClose}>
            <SheetContent className="w-[800px] sm:w-[900px]">
                <SheetHeader>
                    <SheetTitle>Extracted Documents</SheetTitle>
                    {batchState.hasMore && (
                        <div className="text-sm text-muted-foreground">
                            {batchState.processedVideos} of {batchState.totalVideos || '?'} videos processed
                        </div>
                    )}
                </SheetHeader>
                <div className="mt-4 space-y-4">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center space-x-2">
                            <input
                                type="checkbox"
                                checked={selectedCount === documents.length}
                                onChange={(e) => handleSelectAll(e.target.checked)}
                                className="h-4 w-4"
                            />
                            <span className="text-sm">
                                {selectedCount} of {documents.length} selected
                            </span>
                        </div>
                        <Button
                            onClick={handleAddSelectedToBlueband}
                            disabled={selectedCount === 0 || isAddingToBlueband}
                        >
                            {isAddingToBlueband ? (
                                <>
                                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                    Adding...
                                </>
                            ) : (
                                `Add ${selectedCount} to Blueband`
                            )}
                        </Button>
                    </div>
                    <div className="max-h-[calc(100vh-300px)] overflow-y-auto space-y-4">
                        {documents.map((doc, index) => (
                            <Card key={index}>
                                <CardHeader>
                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center space-x-2">
                                            <input
                                                type="checkbox"
                                                checked={doc.isSelected}
                                                onChange={(e) => handleSelectDocument(index, e.target.checked)}
                                                className="h-4 w-4"
                                            />
                                            <CardTitle className="text-base">{doc.title}</CardTitle>
                                        </div>
                                        {doc.isAdded ? (
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                disabled={!!embeddingDoc}
                                                onClick={async () => {
                                                    setEmbeddingDoc(doc.title);
                                                    try {
                                                        await onEmbed(doc);
                                                    } finally {
                                                        setEmbeddingDoc(null);
                                                    }
                                                }}
                                            >
                                                {embeddingDoc === doc.title ? (
                                                    <>
                                                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                                        Embedding...
                                                    </>
                                                ) : (
                                                    'Embed'
                                                )}
                                            </Button>
                                        ) : (
                                            <span className="text-sm text-muted-foreground">Not added</span>
                                        )}
                                    </div>
                                </CardHeader>
                                <CardContent>
                                    <p className="text-sm text-muted-foreground line-clamp-3">
                                        {doc.content}
                                    </p>
                                    {doc.source_url && (
                                        <a
                                            href={doc.source_url}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="text-xs text-primary hover:underline mt-2 inline-block"
                                        >
                                            View source
                                        </a>
                                    )}
                                </CardContent>
                            </Card>
                        ))}
                    </div>
                </div>
                {batchState.hasMore && (
                    <div className="mt-4 flex justify-center">
                        <Button
                            onClick={onContinue}
                            disabled={isAddingToBlueband}
                        >
                            Load More Videos
                        </Button>
                    </div>
                )}
            </SheetContent>
        </Sheet>
    );
}

export function DocumentAdder({ collectionId, collectionName, onClose }: DocumentAdderProps) {
    const { ClanopediaActor, isAuthenticated, login, principal } = useAuth();
    const { toast } = useToast();
    const [activeTab, setActiveTab] = useState<'file' | 'url'>('file');
    const [url, setUrl] = useState('https://www.youtube.com/playlist?list=PLuhDt1vhGcrfg40UUyl1QcAF9e0zvjD8a');
    const [youtubeApiKey, setYoutubeApiKey] = useState('AIzaSyChv-NkSGiqhWgrNj3snUtFkkWL7XnahKY');
    const [isExtracting, setIsExtracting] = useState(false);
    const [isAdmin, setIsAdmin] = useState(false);
    const [extractionProgress, setExtractionProgress] = useState<ExtractionProgress>({
        step: 'idle',
        source: 'file',
        progress: 0,
        message: '',
        extractedDocuments: [],
    });
    const [showPreview, setShowPreview] = useState(false);
    const [extractedDocuments, setExtractedDocuments] = useState<ExtractedDocument[]>([]);
    const [showExtractionSheet, setShowExtractionSheet] = useState(false);
    const [batchState, setBatchState] = useState<BatchExtractionState>({
        hasMore: false,
        processedVideos: 0
    });
    const navigate = useNavigate();

    // Check admin status when component mounts
    useEffect(() => {
        const checkAdminStatus = async () => {
            if (ClanopediaActor && collectionId && principal) {
                try {
                    const adminStatus = await ClanopediaActor.is_admin_check(collectionId, principal);
                    console.log('Admin status check:', {
                        collectionId,
                        principal: principal.toString(),
                        isAdmin: adminStatus
                    });
                    setIsAdmin(adminStatus);
                } catch (error) {
                    console.error('Error checking admin status:', error);
                    setIsAdmin(false);
                }
            }
        };
        checkAdminStatus();
    }, [ClanopediaActor, collectionId, principal]);

    // File drop handling
    const onDrop = useCallback(async (acceptedFiles: File[]) => {
        if (!ClanopediaActor || acceptedFiles.length === 0) return;

        const file = acceptedFiles[0];
        setExtractionProgress(prev => ({
            ...prev,
            step: 'extracting',
            source: 'file',
            progress: 0,
            message: `Processing ${file.name}...`,
        }));

        try {
            // Convert file to Uint8Array
            const arrayBuffer = await file.arrayBuffer();
            const fileData = new Uint8Array(arrayBuffer);

            // Extract content
            const result = await ClanopediaActor.extract_from_file(
                Array.from(fileData),
                file.name,
                collectionId
            );

            if ('Ok' in result) {
                const { documents } = result.Ok;

                setExtractionProgress(prev => ({
                    ...prev,
                    step: 'complete',
                    progress: 100,
                    message: `Successfully extracted ${documents.length} document(s)`,
                }));

                // Show documents in sheet
                setExtractedDocuments(documents.map((doc: any) => ({ ...doc, isSelected: false })));
                setShowExtractionSheet(true);
            } else {
                const errorMessage = typeof result.Err === 'object'
                    ? JSON.stringify(result.Err)
                    : result.Err.toString();
                throw new Error(`Extraction failed: ${errorMessage}`);
            }
        } catch (error) {
            console.error('Extraction error:', error);
            setExtractionProgress(prev => ({
                ...prev,
                step: 'error',
                progress: 0,
                message: error instanceof Error ? error.message : 'Extraction failed',
            }));
            toast({
                title: "Extraction failed",
                description: error instanceof Error ? error.message : "An error occurred",
                variant: "destructive",
            });
        } finally {
            setIsExtracting(false);
        }
    }, [ClanopediaActor, collectionId, toast]);

    const { getRootProps, getInputProps, isDragActive } = useDropzone({
        onDrop,
        accept: {
            'text/plain': ['.txt'],
            'text/markdown': ['.md', '.markdown'],
            'application/pdf': ['.pdf'],
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document': ['.docx'],
        },
        maxFiles: 1,
        disabled: isExtracting,
    });

    // Function to handle embedding a single document
    const handleEmbedDocument = async (doc: ExtractedDocument) => {
        if (!ClanopediaActor) return;

        try {
            const result = await ClanopediaActor.embed_single_document(collectionId, doc);
            if ('Ok' in result) {
                // Update the document's embedded status
                setExtractedDocuments(prev =>
                    prev.map(d =>
                        d.title === doc.title
                            ? { ...d, isEmbedded: true }
                            : d
                    )
                );
                toast({
                    title: "Document embedded",
                    description: `Successfully embedded "${doc.title}"`,
                });
            } else {
                throw new Error('Failed to embed document');
            }
        } catch (error) {
            toast({
                title: "Embedding failed",
                description: error instanceof Error ? error.message : "An error occurred",
                variant: "destructive",
            });
        }
    };

    // Update URL extraction handling
    const handleUrlExtraction = async () => {
        if (!url || !ClanopediaActor) return;

        if (!isAuthenticated) {
            toast({
                title: "Authentication required",
                description: "Please login to extract content from URLs",
                variant: "destructive",
            });
            login();
            return;
        }

        if (!isAdmin) {
            toast({
                title: "Not authorized",
                description: "You must be an admin of this collection to extract content",
                variant: "destructive",
            });
            return;
        }

        setExtractionProgress(prev => ({
            ...prev,
            step: 'extracting',
            source: 'file',
            progress: 0,
            message: 'Extracting content from URL...',
        }));

        try {
            const urlLower = url.toLowerCase();
            let source: ExtractionSource;
            let apiKey: string | undefined;
            let finalUrl = url;

            if (urlLower.includes('youtube.com') || urlLower.includes('youtu.be')) {
                source = 'youtube';
                if (!youtubeApiKey) {
                    throw new Error('YouTube API key is required for YouTube extraction');
                }
                apiKey = youtubeApiKey;
            } else if (urlLower.includes('github.com')) {
                source = 'github';
                if (!isValidGitHubUrl(url)) {
                    throw new Error('Invalid GitHub URL. Please provide a markdown file URL or repository URL');
                }
                finalUrl = convertRepoToReadmeUrl(url);
            } else {
                throw new Error('Only YouTube and GitHub URLs are supported');
            }

            // Extract content
            const result = await ClanopediaActor!.extract_from_url(
                finalUrl,
                collectionId,
                apiKey ? [apiKey] : []
            );

            if ('Ok' in result) {
                const { documents } = result.Ok;

                // Update batch state if it's a YouTube extraction
                if (source === 'youtube') {
                    setBatchState({
                        hasMore: true,
                        nextPageToken: '',
                        totalVideos: 0,
                        processedVideos: 0
                    });
                }

                setExtractionProgress(prev => ({
                    ...prev,
                    step: 'complete',
                    progress: 100,
                    message: `Successfully extracted ${documents.length} document(s)`,
                }));

                // Show extraction sheet with documents
                setExtractedDocuments(documents.map((doc: any) => ({ ...doc, isSelected: false })));
                setShowExtractionSheet(true);
            } else {
                const errorMessage = typeof result.Err === 'object'
                    ? JSON.stringify(result.Err)
                    : result.Err.toString();
                throw new Error(`Extraction failed: ${errorMessage}`);
            }
        } catch (error) {
            console.error('Error extracting from URL:', error);
            setExtractionProgress(prev => ({
                ...prev,
                step: 'error',
                progress: 0,
                message: error instanceof Error ? error.message : 'Failed to extract content',
            }));
            toast({
                title: "Extraction failed",
                description: error instanceof Error ? error.message : "An error occurred",
                variant: "destructive",
            });
        } finally {
            setIsExtracting(false);
        }
    };

    // Add function to handle batch continuation
    const handleContinueExtraction = async () => {
        if (!batchState.hasMore || !batchState.nextPageToken || !ClanopediaActor) return;

        setExtractionProgress(prev => ({
            ...prev,
            step: 'extracting',
            source: 'youtube',
            progress: 0,
            message: 'Extracting more content...',
        }));

        try {
            const result = await ClanopediaActor!.resume_extraction(
                collectionId,
                url,
                youtubeApiKey ? [youtubeApiKey] : []
            );

            if ('Ok' in result) {
                const { documents, extraction_info } = result.Ok;

                // Update batch state
                setBatchState({
                    hasMore: extraction_info.has_more || false,
                    nextPageToken: extraction_info.next_page_token,
                    totalVideos: extraction_info.total_videos,
                    processedVideos: extraction_info.processed_videos
                });

                // Add new documents to existing ones
                setExtractedDocuments(prev => [...prev, ...documents]);

                setExtractionProgress(prev => ({
                    ...prev,
                    step: 'complete',
                    progress: 100,
                    message: `Successfully extracted ${documents.length} more document(s)`,
                }));
            } else {
                throw new Error('Failed to continue extraction');
            }
        } catch (error) {
            toast({
                title: "Extraction failed",
                description: error instanceof Error ? error.message : "An error occurred",
                variant: "destructive",
            });
        }
    };

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <h2 className="text-2xl font-bold">Add Documents to {collectionName}</h2>
                <Button variant="ghost" onClick={onClose}>Close</Button>
            </div>

            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'file' | 'url')}>
                <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="file">Upload File</TabsTrigger>
                    <TabsTrigger value="url">Extract from URL</TabsTrigger>
                </TabsList>

                <TabsContent value="file" className="mt-4">
                    <Card>
                        <CardContent className="pt-6">
                            <div
                                {...getRootProps()}
                                className={cn(
                                    "border-2 border-dashed rounded-lg p-8 text-center cursor-pointer transition-colors",
                                    isDragActive ? "border-primary bg-primary/5" : "border-muted-foreground/25",
                                    isExtracting && "opacity-50 cursor-not-allowed"
                                )}
                            >
                                <input {...getInputProps()} />
                                <FileText className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
                                <p className="text-lg font-medium mb-2">
                                    {isDragActive ? "Drop your file here" : "Drag & drop a file here"}
                                </p>
                                <p className="text-sm text-muted-foreground mb-4">
                                    Supported formats: TXT, MD, PDF, DOCX
                                </p>
                                <Button variant="outline" disabled={isExtracting}>
                                    Select File
                                </Button>
                            </div>
                        </CardContent>
                    </Card>
                </TabsContent>

                <TabsContent value="url" className="mt-4">
                    <Card>
                        <CardContent className="pt-6">
                            <div className="space-y-4">
                                <div className="flex items-center space-x-4">
                                    <Youtube className="h-8 w-8 text-red-500" />
                                    <Github className="h-8 w-8" />
                                    <span className="text-sm text-muted-foreground">
                                        Supported: YouTube playlists, GitHub markdown files
                                    </span>
                                </div>

                                <div className="flex space-x-2">
                                    <Input
                                        placeholder="Paste YouTube or GitHub URL"
                                        value={url}
                                        onChange={(e) => setUrl(e.target.value)}
                                        disabled={isExtracting}
                                    />
                                    <Button
                                        onClick={handleUrlExtraction}
                                        disabled={!url || (url.toLowerCase().includes('youtube') && !youtubeApiKey)}
                                    >
                                        {isExtracting ? (
                                            <>
                                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                                Extracting...
                                            </>
                                        ) : (
                                            'Extract'
                                        )}
                                    </Button>
                                </div>

                                {/* YouTube API Key Input */}
                                {url.toLowerCase().includes('youtube') && (
                                    <div className="mt-4 space-y-2">
                                        <div className="flex items-center space-x-2">
                                            <Input
                                                type="password"
                                                placeholder="Enter YouTube API Key"
                                                value={youtubeApiKey}
                                                onChange={(e) => setYoutubeApiKey(e.target.value)}
                                                disabled={isExtracting}
                                            />
                                        </div>
                                        <p className="text-xs text-muted-foreground">
                                            Get your API key from the{' '}
                                            <a
                                                href="https://console.cloud.google.com/apis/credentials"
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className="text-primary hover:underline"
                                            >
                                                Google Cloud Console
                                            </a>
                                        </p>
                                    </div>
                                )}

                            </div>
                        </CardContent>
                    </Card>
                </TabsContent>
            </Tabs>

            {/* Progress indicator */}
            {extractionProgress.step !== 'idle' && (
                <Card>
                    <CardContent className="pt-6">
                        <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <span className="text-sm font-medium">
                                    {extractionProgress.message}
                                </span>
                                <span className="text-sm text-muted-foreground">
                                    {extractionProgress.progress}%
                                </span>
                            </div>
                            <Progress value={extractionProgress.progress} />
                        </div>
                    </CardContent>
                </Card>
            )}

            {/* Update Extraction Sheet usage */}
            {showExtractionSheet && (
                <ExtractionSheet
                    collectionId={collectionId}
                    documents={extractedDocuments}
                    onClose={() => setShowExtractionSheet(false)}
                    onEmbed={handleEmbedDocument}
                    batchState={batchState}
                    onContinue={handleContinueExtraction}
                    onDocumentsChange={setExtractedDocuments}
                    navigate={navigate}
                />
            )}
        </div>
    );
} 