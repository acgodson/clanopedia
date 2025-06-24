import React, { useState, useEffect } from 'react';
import { Principal } from '@dfinity/principal';
import { GovernanceModel } from 'declarations/clanopedia_backend/clanopedia_backend.did';
import { Button } from '../../atoms/button';
import { Input } from '../../atoms/input';
import { Label } from '../../atoms/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../atoms/select';
import { useToast } from '../../../providers/toast';
import { useAuth } from '../../../providers/useAuth';
import { formatPrincipalWithLabel } from '../../../lib/utils';
import { cn } from '../../../lib/utils';

interface CreateCollectionForm {
  name: string;
  description: string;
  admins: Principal[];
  threshold: number;
  governance_model: GovernanceModel;
  governance_token?: Principal[];
  quorum_threshold: number;
  is_permissionless: boolean;
  sns_governance_canister: string;
}

interface CreateCollectionModalProps {
  onClose: () => void;
  onSubmit: (formData: CreateCollectionForm) => Promise<void>;
}

const DEFAULT_FORM: CreateCollectionForm = {
  name: '',
  description: '',
  admins: [],
  threshold: 1,
  governance_model: { Permissionless: null },
  quorum_threshold: 20,
  is_permissionless: true,
  sns_governance_canister: '',
};

const GOVERNANCE_MODEL_OPTIONS = [
  {
    value: 'Permissionless',
    label: 'Permissionless',
    description: 'Any of the editors can execute proposals without requiring votes',
  },
  {
    value: 'Multisig',
    label: 'Multisig',
    description: 'Proposals require a minimum number of editor approvals to be executed',
  },
  {
    value: 'TokenBased',
    label: 'Token Based',
    description: 'Token holders vote on proposals based on their token balance',
  },
  {
    value: 'SnsIntegrated',
    label: 'SNS Integrated',
    description: 'Proposals are executed through SNS governance',
  },
];

export const CreateCollectionModal: React.FC<CreateCollectionModalProps> = ({ onClose, onSubmit }) => {
  const { toast } = useToast();
  const { ClanopediaActor, principal } = useAuth();
  const [formData, setFormData] = useState<CreateCollectionForm>({
    name: '',
    description: '',
    admins: [],
    threshold: 1,
    governance_model: { Permissionless: null },
    quorum_threshold: 1,
    is_permissionless: true,
    sns_governance_canister: '',
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [newAdminPrincipal, setNewAdminPrincipal] = useState('');
  const [isValidPrincipal, setIsValidPrincipal] = useState(true);
  const [selectedGovernanceModel, setSelectedGovernanceModel] = useState<string>('Permissionless');
  const [snsCanister, setSnsCanister] = useState('');
  const [tokenPrincipal, setTokenPrincipal] = useState('');
  const [quorumThreshold, setQuorumThreshold] = useState(20);
  const [threshold, setThreshold] = useState(2);

  useEffect(() => {
    if (principal) {
      setFormData(prev => ({
        ...prev,
        admins: [principal]
      }));
    }
  }, [principal]);

  const validatePrincipal = (principal: string): boolean => {
    try {
      Principal.fromText(principal);
      return true;
    } catch {
      return false;
    }
  };

  const handleAddAdmin = () => {
    if (!newAdminPrincipal || !isValidPrincipal || !principal) return;

    try {
      const newPrincipal = Principal.fromText(newAdminPrincipal);
      if (newPrincipal.toString() === principal.toString()) {
        toast({
          title: "Cannot add yourself",
          description: "You are already an editor",
          variant: "destructive"
        });
        return;
      }
      
      if (!formData.admins.some(admin => admin.toString() === newPrincipal.toString())) {
        setFormData(prev => ({
          ...prev,
          admins: [...prev.admins, newPrincipal]
        }));
        setNewAdminPrincipal('');
        setIsValidPrincipal(true);
      } else {
        toast({
          title: "Editor already added",
          description: "This principal is already in the editors list",
          variant: "destructive"
        });
      }
    } catch (error) {
      setIsValidPrincipal(false);
    }
  };

  const handleRemoveAdmin = (principalToRemove: Principal) => {
    if (principal && principalToRemove.toString() === principal.toString()) {
      toast({
        title: "Cannot remove yourself",
        description: "You must remain as an editor",
        variant: "destructive"
      });
      return;
    }
    
    setFormData(prev => ({
      ...prev,
      admins: prev.admins.filter(p => p.toString() !== principalToRemove.toString())
    }));
  };

  const handleGovernanceModelChange = (value: string) => {
    const model = { [value]: null } as GovernanceModel;
    setFormData(prev => ({
      ...prev,
      governance_model: model,
      // Reset token-specific fields if not TokenBased
      ...(value !== 'TokenBased' && {
        governance_token: undefined,
        quorum_threshold: 20
      })
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    let snsPrincipalString: string | undefined = undefined;
    if (selectedGovernanceModel === 'SnsIntegrated' && snsCanister) {
      try {
        snsPrincipalString = Principal.fromText(snsCanister).toString();
      } catch (err) {
        toast({
          title: "Invalid SNS Canister Principal",
          description: "Please enter a valid principal ID for the SNS canister.",
          variant: "destructive"
        });
        return;
      }
    }
    let config: any = {
      name: formData.name,
      description: formData.description,
      admins: formData.admins,
      is_permissionless: selectedGovernanceModel === 'Permissionless',
      governance_model: { [selectedGovernanceModel]: null },
      threshold: selectedGovernanceModel === 'Multisig' ? threshold : 0,
      governance_token: selectedGovernanceModel === 'TokenBased' && tokenPrincipal ? [Principal.fromText(tokenPrincipal).toString()] : [],
      quorum_threshold: selectedGovernanceModel === 'TokenBased' ? quorumThreshold : 0,
      sns_governance_canister: selectedGovernanceModel === 'SnsIntegrated' && snsPrincipalString ? [snsPrincipalString] : [],
    };
    try {
      await onSubmit(config);
      onClose();
    } catch (error) {
      console.error('Error creating collection:', error);
      toast({
        title: "Error creating collection",
        description: error instanceof Error ? error.message : "An error occurred",
        variant: "destructive"
      });
    }
  };

  const isFormValid = () => {
    if (!formData.name.trim()) return false;
    if (formData.governance_model && 'TokenBased' in formData.governance_model) {
      if (!formData.governance_token || formData.governance_token.length === 0) return false;
    }
    return true;
  };

  return (
    <div className="w-full max-w-3xl max-h-[90vh] flex flex-col">
      <div className="flex-1 overflow-y-auto p-8 space-y-8">
        <div className="space-y-2">
          <h2 className="text-2xl font-bold">Create New Collection</h2>
          <p className="text-muted-foreground">
            Set up a new collection for your community
          </p>
        </div>

        <div className="space-y-6">
          {/* Name */}
          <div className="space-y-2">
            <Label htmlFor="name">Collection Name</Label>
            <Input
              id="name"
              value={formData.name}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFormData(prev => ({ ...prev, name: e.target.value }))}
              placeholder="Enter collection name"
              className="w-full"
            />
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Input
              id="description"
              value={formData.description}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFormData(prev => ({ ...prev, description: e.target.value }))}
              placeholder="Describe your collection"
              className="w-full"
            />
          </div>

          {/* Editors */}
          <div className="space-y-2">
            <Label>Editors</Label>
            <div className="flex gap-2">
              <Input
                value={newAdminPrincipal}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                  setNewAdminPrincipal(e.target.value);
                  setIsValidPrincipal(true);
                }}
                placeholder="Enter principal ID"
                className={cn("w-full", !isValidPrincipal ? "border-red-500" : "")}
              />
              <Button
                type="button"
                variant="outline"
                onClick={handleAddAdmin}
                disabled={!newAdminPrincipal || !isValidPrincipal}
              >
                Add Editor
              </Button>
            </div>
            {!isValidPrincipal && (
              <p className="text-sm text-red-500">Invalid principal ID</p>
            )}
            {formData.admins.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-2">
                {formData.admins.map((admin) => {
                  const isCurrentUser = principal && admin.toString() === principal.toString();
                  return (
                    <div
                      key={admin.toString()}
                      className={cn(
                        "flex items-center gap-2 px-3 py-1.5 rounded-full text-sm",
                        isCurrentUser
                          ? "bg-primary/10 text-primary border border-primary/20"
                          : "bg-muted text-muted-foreground"
                      )}
                    >
                      <span className="font-mono">
                        {formatPrincipalWithLabel(admin, isCurrentUser ? "You" : undefined)}
                      </span>
                      {!isCurrentUser && (
                        <button
                          type="button"
                          onClick={() => handleRemoveAdmin(admin)}
                          className="text-muted-foreground hover:text-foreground"
                        >
                          ×
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* Governance Model */}
          <div className="space-y-2">
            <Label>Governance Model</Label>
            <Select value={selectedGovernanceModel} onValueChange={setSelectedGovernanceModel}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Select governance model" />
              </SelectTrigger>
              <SelectContent>
                {GOVERNANCE_MODEL_OPTIONS.map(option => (
                  <SelectItem key={option.value} value={option.value}>
                    <div className="flex flex-col">
                      <span className="font-medium">{option.label}</span>
                      <span className="text-xs text-muted-foreground">{option.description}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* SNS Integrated fields */}
          {selectedGovernanceModel === 'SnsIntegrated' && (
            <div className="space-y-2">
              <Label>SNS Governance Canister Principal</Label>
              <Input
                type="text"
                value={snsCanister}
                onChange={e => setSnsCanister(e.target.value)}
                className="w-full"
                required
                placeholder="Enter SNS canister principal"
              />
            </div>
          )}

          {/* TokenBased fields */}
          {selectedGovernanceModel === 'TokenBased' && (
            <>
              <div className="space-y-2">
                <Label>Governance Token Principal</Label>
                <Input
                  type="text"
                  value={tokenPrincipal}
                  onChange={e => setTokenPrincipal(e.target.value)}
                  className="w-full"
                  required
                  placeholder="Enter token canister principal"
                />
              </div>
              <div className="space-y-2">
                <Label>Quorum Threshold (%)</Label>
                <Input
                  type="number"
                  value={quorumThreshold}
                  onChange={e => setQuorumThreshold(Number(e.target.value))}
                  className="w-full"
                  min={1}
                  max={100}
                  required
                  placeholder="Minimum % of total supply to approve"
                />
              </div>
            </>
          )}

          {/* Multisig fields */}
          {selectedGovernanceModel === 'Multisig' && (
            <div className="space-y-2">
              <Label>Threshold (Number of Admins)</Label>
              <Input
                type="number"
                value={threshold}
                onChange={e => setThreshold(Number(e.target.value))}
                className="w-full"
                min={1}
                required
                placeholder="Number of admin approvals required"
              />
            </div>
          )}

          {/* Permissionless toggle */}
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="is_permissionless"
              checked={formData.is_permissionless}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFormData(prev => ({ ...prev, is_permissionless: e.target.checked }))}
              className="rounded border-gray-300"
            />
            <Label htmlFor="is_permissionless">Allow anyone to create proposals</Label>
          </div>
        </div>
      </div>

      {/* Fixed bottom bar */}
      <div className="border-t p-4 bg-background flex justify-end space-x-2">
        <Button
          variant="outline"
          onClick={onClose}
          disabled={isSubmitting}
        >
          Cancel
        </Button>
        <Button
          onClick={handleSubmit}
          disabled={!isFormValid() || isSubmitting}
        >
          {isSubmitting ? "Creating..." : "Create Collection"}
        </Button>
      </div>
    </div>
  );
}; 