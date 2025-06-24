import React, { useEffect, useState } from 'react';
import { Button } from '../atoms/button';
import { Card, CardContent, CardHeader, CardTitle } from '../atoms/card';
import { useAuth } from '../../providers/useAuth';
import { useToast } from '../../providers/toast';
import { Loader2 } from 'lucide-react';

interface DocumentReaderProps {
  documentId: string | null;
  onBack: () => void;
  collectionName: string;
  collectionId: string;
  searchQuery?: string;
}

interface DocumentMetadata {
  id: string;
  title: string;
  content: string;
  author?: string;
  source_url?: string;
  timestamp: number;
}

export function DocumentReader({ documentId, onBack, collectionName, collectionId, searchQuery }: DocumentReaderProps) {
  const { BluebandActor } = useAuth();
  const { toast } = useToast();
  const [isLoading, setIsLoading] = useState(true);
  const [relevantSections, setRelevantSections] = useState<string[]>([]);
  const [documentContent, setDocumentContent] = useState<string>('');
  const [documentTitle, setDocumentTitle] = useState<string>('');
  const [sourceUrl, setSourceUrl] = useState<string | null>(null);

  useEffect(() => {
    const fetchDocument = async () => {
      if (!BluebandActor || !collectionId || !documentId) return;

      setIsLoading(true);
      try {
        // Get the document metadata
        const metadata = await BluebandActor.get_document(collectionId, documentId);
        if (metadata) {
          setSourceUrl(metadata.source_url || null);
          setDocumentTitle(metadata.title || 'Untitled');
        }

        // Get the full document content
        const content = await BluebandActor.get_document_content(collectionId, documentId);
        if (content) {
          const contentText = content.toString();
          setDocumentContent(contentText);
          
          // Only split if we have a search query
          if (searchQuery) {
            const sections = findRelevantSections(contentText, searchQuery);
            setRelevantSections(sections);
          }
        } else {
          throw new Error('Document not found');
        }
      } catch (error) {
        console.error('Error fetching document:', error);
        toast({
          title: "Error",
          description: error instanceof Error ? error.message : "Failed to fetch document",
          variant: "destructive",
        });
      } finally {
        setIsLoading(false);
      }
    };

    fetchDocument();
  }, [BluebandActor, collectionId, documentId, searchQuery, toast]);

  const findRelevantSections = (content: string, query: string): string[] => {
    const sections: string[] = [];
    const lines = content.split('\n');
    let currentSection = '';
    
    for (const line of lines) {
      if (line.startsWith('#') || line.startsWith('##') || line.startsWith('###')) {
        currentSection = line;
      } else if (currentSection && line.toLowerCase().includes(query.toLowerCase())) {
        sections.push(currentSection);
      }
    }
    
    return sections;
  };

  const handleAskAboutDocument = () => {
    // TODO: Implement document Q&A functionality
    toast({
      title: "Coming soon",
      description: "Document Q&A functionality will be available soon",
    });
  };

  if (!documentId) {
    return <div className="text-center py-10">No document selected.</div>;
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-200px)]">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  if (!documentContent) {
    return <div className="text-center py-10">Document not found.</div>;
  }

  return (
    <div className="space-y-8">
      {/* Back button in its own row */}
      <div className="w-full">
        <Button variant="ghost" onClick={onBack} className="text-muted-foreground hover:text-foreground p-0 h-auto">
          ← Back to Results
        </Button>
      </div>

      {/* Title in its own row */}
      <div className="w-full">
        <h1 className="text-xl font-bold text-foreground break-words">
          {collectionName}: {documentTitle}
        </h1>
      </div>

      <Card className="w-full">
        <CardContent className="p-6">
          <div 
            className="prose prose-sm max-w-none text-foreground whitespace-pre-wrap"
          >
            {documentContent}
          </div>

          {relevantSections.length > 0 && (
            <div className="mt-8 space-y-4">
              <h3 className="text-lg font-semibold">🎯 Relevant Sections:</h3>
              <ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground ml-4">
                {relevantSections.map((section, index) => (
                  <li key={index}>{section}</li>
                ))}
              </ul>
            </div>
          )}

          {sourceUrl && (
            <div className="mt-4">
              <a 
                href={sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                View original source
              </a>
            </div>
          )}

          <div className="flex items-center space-x-2 mt-8">
            <input
              type="text"
              placeholder="💬 Ask about this document:"
              className="flex-1 px-4 py-2 rounded-md border border-input bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
            />
            <Button onClick={handleAskAboutDocument}>→</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
} 