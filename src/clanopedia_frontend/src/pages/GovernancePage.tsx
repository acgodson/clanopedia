import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/atoms/card";
import { Button } from "../components/atoms/button";

export function GovernancePage() {
  return (
    <div className="min-h-screen bg-background p-8">
      <div className="mx-auto max-w-4xl space-y-8">
        <header className="text-center mb-12">
          <h1 className="text-3xl md:text-4xl font-extrabold tracking-tight text-foreground mb-4">
            🗳️ Clanopedia Governance
          </h1>
          <p className="text-muted-foreground text-lg">
            Participate in the democratic curation of knowledge.
          </p>
        </header>

        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <Card className="w-full">
            <CardHeader>
              <CardTitle>Proposal Types</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm text-muted-foreground">
              <p>• Embed Document</p>
              <p>• Batch Embed Documents</p>
              <p>• Add/Remove Admin</p>
              <p>• Change Threshold</p>
              <p>• Update Quorum</p>
              <p>• Transfer Genesis</p>
            </CardContent>
          </Card>

          <Card className="w-full">
            <CardHeader>
              <CardTitle>Proposal Lifecycle</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm text-muted-foreground">
              <p>• **Created**: A new proposal is initiated.</p>
              <p>• **Voting**: Community members cast their votes (default 7 days).</p>
              <p>• **Executed**: Proposal passes, changes are applied.</p>
              <p>• **Rejected**: Proposal fails to meet threshold.</p>
              <p>• **Expired**: Voting period ends without resolution.</p>
            </CardContent>
          </Card>

          <Card className="w-full">
            <CardHeader>
              <CardTitle>Your Active Proposals</CardTitle>
            </CardHeader>
            <CardContent className="text-center text-muted-foreground">
              <p className="mb-4">No active proposals yet.</p>
              <Button>Create New Proposal</Button>
            </CardContent>
          </Card>
        </div>

        {/* Future sections for ongoing proposals, voting activity, etc. */}

      </div>
    </div>
  );
} 