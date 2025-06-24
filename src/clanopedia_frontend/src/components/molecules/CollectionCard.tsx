import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../atoms/card";
import { Button } from "../atoms/button";
import { Link } from 'react-router-dom';

interface CollectionCardProps {
  id: string;
  name: string;
  description: string;
  memberCount: number;
  documentCount: number;
  governanceModel: string;
  proposalCount?: number;
}

export function CollectionCard({
  id,
  name,
  description,
  memberCount,
  documentCount,
  governanceModel,
  proposalCount,
}: CollectionCardProps) {
  console.log({'id': id, 'name': name})
  return (
    <Link to={`/collections/${id}`} className="block">
      <Card className="w-full hover:shadow-lg transition-shadow duration-200">
        <CardHeader>
          <CardTitle>{name}</CardTitle>
          <CardDescription>{description}</CardDescription>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-2 text-sm text-muted-foreground">
          <div className="flex items-center">
            <span className="mr-1">👥</span> {memberCount} editors
          </div>
          <div className="flex items-center">
            <span className="mr-1">📄</span> {documentCount} docs
          </div>
          <div className="flex items-center col-span-2">
            <span className="mr-1">🗳️</span> {governanceModel}
            {proposalCount !== undefined && (
              <span className="ml-2 text-primary font-medium">⚡ Proposals: {proposalCount}</span>
            )}
          </div>
        </CardContent>
      </Card>
    </Link>
  );
} 