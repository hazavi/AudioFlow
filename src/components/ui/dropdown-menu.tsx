import * as Dropdown from "@radix-ui/react-dropdown-menu";
export const DropdownMenu = Dropdown.Root;
export const DropdownMenuTrigger = Dropdown.Trigger;
export function DropdownMenuContent({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <Dropdown.Portal>
      <Dropdown.Content className="dropdown-menu" align="end" sideOffset={6}>
        {children}
      </Dropdown.Content>
    </Dropdown.Portal>
  );
}
export function DropdownMenuItem({
  children,
  onSelect,
}: {
  children: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <Dropdown.Item className="dropdown-item" onSelect={onSelect}>
      {children}
    </Dropdown.Item>
  );
}
